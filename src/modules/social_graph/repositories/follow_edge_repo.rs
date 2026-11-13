use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::modules::{
    social_feed::repositories::profile_projection_repo::{
        ensure_projection_exists, resolve_target_user_id,
    },
    social_graph::{
        error::SocialGraphError,
        models::{
            ActorSummaryRow, EdgePairState, FollowEdgeStatus, FollowMutationOutcome, TargetUserRow,
        },
    },
};

#[derive(FromRow)]
struct RawMutationRow {
    changed: bool,
    resulting_status: Option<String>,
    target_followers_count: i32,
    target_following_count: i32,
    actor_following_count: i32,
}

#[derive(FromRow)]
struct PairEdgeRow {
    follower_id: Uuid,
    following_id: Uuid,
    status: String,
}

pub async fn resolve_target_user(
    pool: &PgPool,
    target: &str,
) -> Result<TargetUserRow, SocialGraphError> {
    let uid = resolve_target_user_id(pool, target, None)
        .await
        .map_err(|_| SocialGraphError::NotFound("Pengguna target tidak ditemukan.".to_string()))?;
    let _ = ensure_projection_exists(pool, uid).await;

    sqlx::query_as::<_, TargetUserRow>(
        "SELECT user_id, is_private_account, followers_count, following_count
         FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| SocialGraphError::NotFound("Proyeksi profil target tidak ditemukan.".to_string()))
}

pub async fn fetch_actor_summary(
    pool: &PgPool,
    actor_id: Uuid,
) -> Result<ActorSummaryRow, SocialGraphError> {
    let _ = ensure_projection_exists(pool, actor_id).await;
    sqlx::query_as::<_, ActorSummaryRow>(
        "SELECT user_id, username, display_name, avatar_url, following_count
         FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(actor_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| SocialGraphError::NotFound("Akun aktor tidak ditemukan.".to_string()))
}

pub async fn fetch_pair_edge_state(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<EdgePairState, SocialGraphError> {
    let rows = sqlx::query_as::<_, PairEdgeRow>(
        "SELECT follower_id, following_id, status::text AS status
         FROM user_follows
         WHERE (follower_id = $1 AND following_id = $2)
            OR (follower_id = $2 AND following_id = $1)",
    )
    .bind(viewer_id)
    .bind(target_id)
    .fetch_all(pool)
    .await?;

    let mut state = EdgePairState::default();
    for r in rows {
        if r.follower_id == viewer_id && r.following_id == target_id {
            match r.status.as_str() {
                "ACCEPTED" => state.out_accepted = true,
                "PENDING" => state.out_pending = true,
                _ => {}
            }
        } else if r.follower_id == target_id && r.following_id == viewer_id {
            match r.status.as_str() {
                "ACCEPTED" => state.in_accepted = true,
                "PENDING" => state.in_pending = true,
                _ => {}
            }
        }
    }
    Ok(state)
}

pub async fn follow_user_idempotent(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
    is_target_private: bool,
) -> Result<FollowMutationOutcome, SocialGraphError> {
    let desired = if is_target_private { "PENDING" } else { "ACCEPTED" };
    let row = sqlx::query_as::<_, RawMutationRow>(
        "WITH inserted AS (
            INSERT INTO user_follows (follower_id, following_id, status, accepted_at)
            SELECT $1, $2, $3::follow_edge_status, CASE WHEN $3 = 'ACCEPTED' THEN NOW() ELSE NULL END
            ON CONFLICT (follower_id, following_id) DO NOTHING
            RETURNING status::text AS status
        ),
        upd_actor AS (
            UPDATE user_profile_projections
            SET following_count = following_count + 1, updated_at = NOW()
            WHERE user_id = $1 AND EXISTS (SELECT 1 FROM inserted WHERE status = 'ACCEPTED')
            RETURNING following_count
        ),
        upd_target AS (
            UPDATE user_profile_projections
            SET followers_count = followers_count + CASE WHEN EXISTS (SELECT 1 FROM inserted WHERE status = 'ACCEPTED') THEN 1 ELSE 0 END,
                pending_follow_requests_count = pending_follow_requests_count + CASE WHEN EXISTS (SELECT 1 FROM inserted WHERE status = 'PENDING') THEN 1 ELSE 0 END,
                updated_at = NOW()
            WHERE user_id = $2 AND EXISTS (SELECT 1 FROM inserted)
            RETURNING followers_count, following_count
        )
        SELECT EXISTS(SELECT 1 FROM inserted) AS changed,
               COALESCE((SELECT status FROM inserted LIMIT 1), (SELECT status::text FROM user_follows WHERE follower_id = $1 AND following_id = $2 LIMIT 1), $3) AS resulting_status,
               COALESCE((SELECT followers_count FROM upd_target), (SELECT followers_count FROM user_profile_projections WHERE user_id = $2), 0) AS target_followers_count,
               COALESCE((SELECT following_count FROM upd_target), (SELECT following_count FROM user_profile_projections WHERE user_id = $2), 0) AS target_following_count,
               COALESCE((SELECT following_count FROM upd_actor), (SELECT following_count FROM user_profile_projections WHERE user_id = $1), 0) AS actor_following_count",
    )
    .bind(follower_id).bind(following_id).bind(desired).fetch_one(pool).await?;

    let resulting_status = match row.resulting_status.as_deref() {
        Some("PENDING") => Some(FollowEdgeStatus::Pending),
        _ => Some(FollowEdgeStatus::Accepted),
    };
    Ok(FollowMutationOutcome {
        changed: row.changed,
        resulting_status,
        target_followers_count: row.target_followers_count,
        target_following_count: row.target_following_count,
        actor_following_count: row.actor_following_count,
    })
}

pub async fn unfollow_user_idempotent(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
) -> Result<FollowMutationOutcome, SocialGraphError> {
    let row = sqlx::query_as::<_, RawMutationRow>(
        "WITH deleted AS (
            DELETE FROM user_follows WHERE follower_id = $1 AND following_id = $2
            RETURNING status::text AS status
        ),
        upd_actor AS (
            UPDATE user_profile_projections
            SET following_count = GREATEST(0, following_count - 1), updated_at = NOW()
            WHERE user_id = $1 AND EXISTS (SELECT 1 FROM deleted WHERE status = 'ACCEPTED')
            RETURNING following_count
        ),
        upd_target AS (
            UPDATE user_profile_projections
            SET followers_count = GREATEST(0, followers_count - CASE WHEN EXISTS (SELECT 1 FROM deleted WHERE status = 'ACCEPTED') THEN 1 ELSE 0 END),
                pending_follow_requests_count = GREATEST(0, pending_follow_requests_count - CASE WHEN EXISTS (SELECT 1 FROM deleted WHERE status = 'PENDING') THEN 1 ELSE 0 END),
                updated_at = NOW()
            WHERE user_id = $2 AND EXISTS (SELECT 1 FROM deleted)
            RETURNING followers_count, following_count
        )
        SELECT EXISTS(SELECT 1 FROM deleted) AS changed,
               (SELECT status FROM deleted LIMIT 1) AS resulting_status,
               COALESCE((SELECT followers_count FROM upd_target), (SELECT followers_count FROM user_profile_projections WHERE user_id = $2), 0) AS target_followers_count,
               COALESCE((SELECT following_count FROM upd_target), (SELECT following_count FROM user_profile_projections WHERE user_id = $2), 0) AS target_following_count,
               COALESCE((SELECT following_count FROM upd_actor), (SELECT following_count FROM user_profile_projections WHERE user_id = $1), 0) AS actor_following_count",
    )
    .bind(follower_id).bind(following_id).fetch_one(pool).await?;

    Ok(FollowMutationOutcome {
        changed: row.changed,
        resulting_status: None,
        target_followers_count: row.target_followers_count,
        target_following_count: row.target_following_count,
        actor_following_count: row.actor_following_count,
    })
}
