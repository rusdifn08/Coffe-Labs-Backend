use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::modules::social_feed::{
    error::SocialFeedError,
    models::{BadgeCatalogMiniRow, UserProfileProjectionRow},
};

pub use super::profile_write_repo::{award_xp_on_projection, update_projection_profile};

pub async fn resolve_target_user_id(
    pool: &PgPool,
    target: &str,
    viewer_id: Option<Uuid>,
) -> Result<Uuid, SocialFeedError> {
    let clean = target.trim().trim_start_matches('@').trim();
    if clean.is_empty() {
        return Err(SocialFeedError::ProfileNotFound);
    }
    if clean.eq_ignore_ascii_case("me") {
        return viewer_id.ok_or(SocialFeedError::ProfileNotFound);
    }
    if let Ok(uid) = Uuid::parse_str(clean) {
        return Ok(uid);
    }

    if let Some(uid) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE LOWER(username) = LOWER($1) LIMIT 1",
    )
    .bind(clean)
    .fetch_optional(pool)
    .await?
    {
        return Ok(uid);
    }

    if let Some(uid) = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM user_profile_projections WHERE LOWER(username) = LOWER($1) LIMIT 1",
    )
    .bind(clean)
    .fetch_optional(pool)
    .await?
    {
        return Ok(uid);
    }

    let by_name: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(full_name) = LOWER($1) LIMIT 1",
    )
    .bind(clean)
    .fetch_optional(pool)
    .await?;

    by_name.ok_or(SocialFeedError::ProfileNotFound)
}

pub async fn ensure_projection_exists(
    pool: &PgPool,
    target_user_id: Uuid,
) -> Result<UserProfileProjectionRow, SocialFeedError> {
    if let Some(existing) = sqlx::query_as::<_, UserProfileProjectionRow>(
        "SELECT * FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(existing);
    }

    let backfill_res = sqlx::query(
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
        ON CONFLICT DO NOTHING"#,
    )
    .bind(target_user_id)
    .execute(pool)
    .await;

    if let Err(err) = backfill_res {
        warn!(user_id = %target_user_id, error = %err, "Projection backfill with gamification join failed; trying bare users backfill");
        let _ = sqlx::query(
            r#"INSERT INTO user_profile_projections (
                user_id, username, display_name, bio, avatar_url, account_tier
            )
            SELECT
                u.id,
                LEFT(u.username, 40),
                LEFT(COALESCE(NULLIF(u.full_name, ''), u.username), 80),
                'Barista & V60 Enthusiast di Coffee Labs',
                u.avatar_url,
                'Trial'
            FROM users u
            WHERE u.id = $1
            ON CONFLICT DO NOTHING"#,
        )
        .bind(target_user_id)
        .execute(pool)
        .await;
    }

    sqlx::query_as::<_, UserProfileProjectionRow>(
        "SELECT * FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(target_user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(SocialFeedError::ProfileNotFound)
}

pub async fn fetch_badges_for_ids(
    pool: &PgPool,
    badge_ids: &[u32],
) -> Result<Vec<BadgeCatalogMiniRow>, SocialFeedError> {
    if badge_ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids_i32: Vec<i32> = badge_ids.iter().map(|id| *id as i32).collect();
    match sqlx::query_as::<_, BadgeCatalogMiniRow>(
        r#"SELECT id, slug, title, rarity, rive_artboard
           FROM achievements_catalog
           WHERE id = ANY($1::int[])
           ORDER BY xp_reward DESC
           LIMIT 5"#,
    )
    .bind(&ids_i32)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => Ok(rows),
        Err(err) => {
            warn!(error = %err, "Gagal memuat katalog badge; mengembalikan daftar kosong");
            Ok(Vec::new())
        }
    }
}
