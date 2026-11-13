use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_feed::{
    dto::UpdateMyProfileRequest,
    error::SocialFeedError,
    models::{
        barista_rank_for_level, compute_level_progress_ratio, level_from_total_xp,
        BadgeCatalogMiniRow, UserProfileProjectionRow,
    },
};

pub async fn resolve_target_user_id(
    pool: &PgPool,
    target: &str,
    viewer_id: Option<Uuid>,
) -> Result<Uuid, SocialFeedError> {
    let clean = target.trim().trim_start_matches('@');
    if clean.eq_ignore_ascii_case("me") {
        return viewer_id.ok_or(SocialFeedError::ProfileNotFound);
    }
    if let Ok(uid) = Uuid::parse_str(clean) {
        return Ok(uid);
    }

    let found: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = LOWER($1) LIMIT 1",
    )
    .bind(clean)
    .fetch_optional(pool)
    .await?;

    found.ok_or(SocialFeedError::ProfileNotFound)
}

pub async fn ensure_projection_exists(
    pool: &PgPool,
    target_user_id: Uuid,
) -> Result<UserProfileProjectionRow, SocialFeedError> {
    sqlx::query(
        r#"INSERT INTO user_profile_projections (
            user_id, username, display_name, bio, avatar_url, account_tier,
            total_xp, barista_level, barista_rank_title, current_streak_days, unlocked_badges_bitmap
        )
        SELECT
            u.id,
            LEFT(u.username, 40),
            LEFT(COALESCE(NULLIF(u.full_name, ''), u.username), 80),
            COALESCE(u.bio, 'Barista & V60 Enthusiast di Coffee Labs'),
            u.avatar_url,
            INITCAP(COALESCE(u.tier, 'trial')),
            COALESCE(g.total_xp, 0),
            COALESCE(g.current_level, 1),
            CASE
                WHEN COALESCE(g.current_level, 1) >= 80 THEN 'WBrC Grandmaster'
                WHEN COALESCE(g.current_level, 1) >= 50 THEN 'Head Roaster & Master'
                WHEN COALESCE(g.current_level, 1) >= 25 THEN 'Sensory Alchemist'
                WHEN COALESCE(g.current_level, 1) >= 10 THEN 'Pour-Over Artisan'
                ELSE 'Home Brewer'
            END,
            COALESCE(g.current_streak, 0),
            g.unlocked_badges_bitmap
        FROM users u
        LEFT JOIN gamification_profiles g ON g.user_id = u.id
        WHERE u.id = $1
        ON CONFLICT (user_id) DO NOTHING"#,
    )
    .bind(target_user_id)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, UserProfileProjectionRow>(
        "SELECT * FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(SocialFeedError::ProfileNotFound)?;

    Ok(row)
}

pub async fn fetch_badges_for_ids(
    pool: &PgPool,
    badge_ids: &[u32],
) -> Result<Vec<BadgeCatalogMiniRow>, SocialFeedError> {
    if badge_ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids_i32: Vec<i32> = badge_ids.iter().map(|id| *id as i32).collect();
    let rows = sqlx::query_as::<_, BadgeCatalogMiniRow>(
        r#"SELECT id, slug, title, rarity, rive_artboard
           FROM achievements_catalog
           WHERE id = ANY($1::int[])
           ORDER BY xp_reward DESC
           LIMIT 5"#,
    )
    .bind(&ids_i32)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

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
