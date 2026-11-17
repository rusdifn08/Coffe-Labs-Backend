use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_feed::{
    dto::UpdateMyProfileRequest,
    error::SocialFeedError,
    models::{barista_rank_for_level, compute_level_progress_ratio, level_from_total_xp},
    repositories::profile_projection_repo::ensure_projection_exists,
};

pub async fn update_projection_profile(
    pool: &PgPool,
    user_id: Uuid,
    req: &UpdateMyProfileRequest,
    clean_display_name: Option<&str>,
    clean_bio: Option<&str>,
) -> Result<(), SocialFeedError> {
    let snapshot_json = req
        .recent_brew_snapshot
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| SocialFeedError::BadRequest("Snapshot seduh tidak valid".into()))?;

    ensure_projection_exists(pool, user_id).await?;

    sqlx::query(
        r#"UPDATE user_profile_projections
           SET display_name = COALESCE($2, display_name),
               bio = COALESCE($3, bio),
               avatar_url = COALESCE($4, avatar_url),
               banner_url = COALESCE($5, banner_url),
               recent_brew_snapshot = COALESCE($6, recent_brew_snapshot),
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .bind(clean_display_name)
    .bind(clean_bio)
    .bind(req.avatar_url.as_deref())
    .bind(req.banner_url.as_deref())
    .bind(snapshot_json)
    .execute(pool)
    .await?;

    if clean_bio.is_some() || clean_display_name.is_some() || req.avatar_url.is_some() {
        let _ = sqlx::query(
            r#"UPDATE users
               SET full_name = COALESCE($2, full_name),
                   bio = COALESCE($3, bio),
                   avatar_url = COALESCE($4, avatar_url),
                   updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(user_id)
        .bind(clean_display_name)
        .bind(clean_bio)
        .bind(req.avatar_url.as_deref())
        .execute(pool)
        .await;
    }

    Ok(())
}

pub async fn award_xp_on_projection(
    pool: &PgPool,
    user_id: Uuid,
    delta_xp: i64,
) -> Result<(i64, i32), SocialFeedError> {
    let proj = ensure_projection_exists(pool, user_id).await?;
    let new_xp = (proj.total_xp + delta_xp).max(0);
    let new_level = level_from_total_xp(new_xp);
    let new_rank = barista_rank_for_level(new_level);
    let progress = compute_level_progress_ratio(new_level, new_xp);

    sqlx::query(
        r#"UPDATE user_profile_projections
           SET total_xp = $2,
               barista_level = $3,
               barista_rank_title = $4,
               level_progress_ratio = $5,
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .bind(new_xp)
    .bind(new_level)
    .bind(new_rank)
    .bind(progress)
    .execute(pool)
    .await?;

    Ok((new_xp, new_level))
}
