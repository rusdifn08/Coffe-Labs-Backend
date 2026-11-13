use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::modules::{
    social_feed::repositories::profile_projection_repo::ensure_projection_exists,
    social_graph::{error::SocialGraphError, models::PrivacySettingRow},
};

#[derive(FromRow)]
pub struct AcceptRejectOutcomeRow {
    pub changed: bool,
    pub followers_count: i32,
    pub pending_follow_requests_count: i32,
}

pub async fn accept_follow_request_idempotent(
    pool: &PgPool,
    target_id: Uuid,
    requester_id: Uuid,
) -> Result<AcceptRejectOutcomeRow, SocialGraphError> {
    let _ = ensure_projection_exists(pool, target_id).await;
    let _ = ensure_projection_exists(pool, requester_id).await;

    Ok(sqlx::query_as::<_, AcceptRejectOutcomeRow>(
        "WITH accepted AS (
            UPDATE user_follows
            SET status = 'ACCEPTED'::follow_edge_status, accepted_at = NOW()
            WHERE follower_id = $2 AND following_id = $1 AND status = 'PENDING'
            RETURNING follower_id
        ),
        upd_requester AS (
            UPDATE user_profile_projections
            SET following_count = following_count + 1, updated_at = NOW()
            WHERE user_id = $2 AND EXISTS (SELECT 1 FROM accepted)
        ),
        upd_target AS (
            UPDATE user_profile_projections
            SET followers_count = followers_count + 1,
                pending_follow_requests_count = GREATEST(0, pending_follow_requests_count - 1),
                updated_at = NOW()
            WHERE user_id = $1 AND EXISTS (SELECT 1 FROM accepted)
            RETURNING followers_count, pending_follow_requests_count
        )
        SELECT EXISTS(SELECT 1 FROM accepted) AS changed,
               COALESCE((SELECT followers_count FROM upd_target), (SELECT followers_count FROM user_profile_projections WHERE user_id = $1), 0) AS followers_count,
               COALESCE((SELECT pending_follow_requests_count FROM upd_target), (SELECT pending_follow_requests_count FROM user_profile_projections WHERE user_id = $1), 0) AS pending_follow_requests_count",
    )
    .bind(target_id).bind(requester_id).fetch_one(pool).await?)
}

pub async fn reject_follow_request_idempotent(
    pool: &PgPool,
    target_id: Uuid,
    requester_id: Uuid,
) -> Result<AcceptRejectOutcomeRow, SocialGraphError> {
    let _ = ensure_projection_exists(pool, target_id).await;

    Ok(sqlx::query_as::<_, AcceptRejectOutcomeRow>(
        "WITH rejected AS (
            DELETE FROM user_follows
            WHERE follower_id = $2 AND following_id = $1 AND status = 'PENDING'
            RETURNING follower_id
        ),
        upd_target AS (
            UPDATE user_profile_projections
            SET pending_follow_requests_count = GREATEST(0, pending_follow_requests_count - 1),
                updated_at = NOW()
            WHERE user_id = $1 AND EXISTS (SELECT 1 FROM rejected)
            RETURNING followers_count, pending_follow_requests_count
        )
        SELECT EXISTS(SELECT 1 FROM rejected) AS changed,
               COALESCE((SELECT followers_count FROM upd_target), (SELECT followers_count FROM user_profile_projections WHERE user_id = $1), 0) AS followers_count,
               COALESCE((SELECT pending_follow_requests_count FROM upd_target), (SELECT pending_follow_requests_count FROM user_profile_projections WHERE user_id = $1), 0) AS pending_follow_requests_count",
    )
    .bind(target_id).bind(requester_id).fetch_one(pool).await?)
}

pub async fn fetch_privacy_setting(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<PrivacySettingRow, SocialGraphError> {
    let _ = ensure_projection_exists(pool, user_id).await;
    sqlx::query_as::<_, PrivacySettingRow>(
        "SELECT is_private_account, pending_follow_requests_count
         FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| SocialGraphError::NotFound("Proyeksi profil tidak ditemukan.".to_string()))
}

pub async fn update_privacy_setting(
    pool: &PgPool,
    user_id: Uuid,
    is_private: bool,
) -> Result<PrivacySettingRow, SocialGraphError> {
    let _ = ensure_projection_exists(pool, user_id).await;
    sqlx::query_as::<_, PrivacySettingRow>(
        "UPDATE user_profile_projections
         SET is_private_account = $2, updated_at = NOW()
         WHERE user_id = $1
         RETURNING is_private_account, pending_follow_requests_count",
    )
    .bind(user_id)
    .bind(is_private)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| SocialGraphError::NotFound("Proyeksi profil tidak ditemukan.".to_string()))
}
