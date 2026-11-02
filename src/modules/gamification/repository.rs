use chrono::NaiveDate;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::gamification::{
        models::{
            AchievementCatalogRow, BrewVerificationSessionRow, GamificationProfileRow,
            LeaderboardRow,
        },
        services::xp_calculator::{decode_badges_bitmap, level_title_for},
    },
};

pub async fn get_or_create_profile(pool: &PgPool, user_id: Uuid, default_freezes: i32, timezone: &str) -> AppResult<GamificationProfileRow> {
    let row = sqlx::query_as::<_, GamificationProfileRow>(
        r#"INSERT INTO gamification_profiles (user_id, streak_freezes_available, timezone) VALUES ($1, $2, $3)
           ON CONFLICT (user_id) DO UPDATE SET updated_at = gamification_profiles.updated_at
           RETURNING user_id, total_xp, current_level, current_streak, longest_streak, streak_freezes_available, last_brew_local_date, timezone, unlocked_badges_bitmap, updated_at"#,
    ).bind(user_id).bind(default_freezes).bind(timezone).fetch_one(pool).await?;
    Ok(row)
}

pub async fn list_catalog(pool: &PgPool) -> AppResult<Vec<AchievementCatalogRow>> {
    Ok(sqlx::query_as::<_, AchievementCatalogRow>(
        "SELECT id, slug, title, category, description, criteria_json, xp_reward, rive_artboard, rarity, tier_required FROM achievements_catalog ORDER BY id ASC",
    ).fetch_all(pool).await?)
}

pub async fn insert_brew_session(pool: &PgPool, user_id: Uuid, recipe_id: Uuid, target_duration_sec: i32) -> AppResult<BrewVerificationSessionRow> {
    Ok(sqlx::query_as::<_, BrewVerificationSessionRow>(
        r#"INSERT INTO brew_verification_sessions (user_id, recipe_id, target_duration_sec, status) VALUES ($1, $2, $3, 'STARTED')
           RETURNING id, user_id, recipe_id, target_duration_sec, started_at, completed_at, is_valid, status"#,
    ).bind(user_id).bind(recipe_id).bind(target_duration_sec).fetch_one(pool).await?)
}

pub async fn find_brew_session_for_update<'a>(tx: &mut Transaction<'a, Postgres>, session_id: Uuid, user_id: Uuid) -> AppResult<Option<BrewVerificationSessionRow>> {
    Ok(sqlx::query_as::<_, BrewVerificationSessionRow>(
        "SELECT id, user_id, recipe_id, target_duration_sec, started_at, completed_at, is_valid, status FROM brew_verification_sessions WHERE id = $1 AND user_id = $2 FOR UPDATE",
    ).bind(session_id).bind(user_id).fetch_optional(&mut **tx).await?)
}

pub async fn mark_session_completed<'a>(tx: &mut Transaction<'a, Postgres>, session_id: Uuid, is_valid: bool, status: &str) -> AppResult<()> {
    sqlx::query("UPDATE brew_verification_sessions SET completed_at = NOW(), is_valid = $1, status = $2 WHERE id = $3")
        .bind(is_valid).bind(status).bind(session_id).execute(&mut **tx).await?;
    Ok(())
}

pub async fn sum_today_xp<'a>(tx: &mut Transaction<'a, Postgres>, user_id: Uuid) -> AppResult<i64> {
    let sum: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(final_xp), 0)::bigint FROM xp_ledger_transactions WHERE user_id = $1 AND created_at >= NOW() - INTERVAL '24 hours'",
    ).bind(user_id).fetch_optional(&mut **tx).await?;
    Ok(sum.unwrap_or(0))
}

#[allow(clippy::too_many_arguments)]
pub async fn append_xp_ledger<'a>(
    tx: &mut Transaction<'a, Postgres>,
    ulid_id: &str,
    user_id: Uuid,
    action_type: &str,
    base_xp: i32,
    tier_multiplier: f64,
    final_xp: i32,
    reference_id: Option<Uuid>,
    idempotency_key: &str,
) -> AppResult<bool> {
    let res = sqlx::query(
        r#"INSERT INTO xp_ledger_transactions (
            id, user_id, action_type, base_xp, tier_multiplier, final_xp, reference_id, idempotency_key
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (idempotency_key) DO NOTHING"#,
    )
    .bind(ulid_id)
    .bind(user_id)
    .bind(action_type)
    .bind(base_xp)
    .bind(tier_multiplier)
    .bind(final_xp)
    .bind(reference_id)
    .bind(idempotency_key)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() > 0)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_profile_tx<'a>(
    tx: &mut Transaction<'a, Postgres>,
    user_id: Uuid,
    total_xp: i64,
    current_level: i32,
    current_streak: i32,
    longest_streak: i32,
    freezes_available: i32,
    last_brew_date: NaiveDate,
    timezone: &str,
    bitmap_bytes: &[u8],
) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE gamification_profiles
           SET total_xp = $2, current_level = $3, current_streak = $4,
               longest_streak = $5, streak_freezes_available = $6,
               last_brew_local_date = $7, timezone = $8,
               unlocked_badges_bitmap = $9, updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .bind(total_xp)
    .bind(current_level)
    .bind(current_streak)
    .bind(longest_streak)
    .bind(freezes_available)
    .bind(last_brew_date)
    .bind(timezone)
    .bind(bitmap_bytes)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn query_leaderboard(pool: &PgPool, weekly_only: bool, limit: i64) -> AppResult<Vec<LeaderboardRow>> {
    #[derive(sqlx::FromRow)]
    struct RawLeaderRow {
        user_id: Uuid,
        username: String,
        full_name: String,
        avatar_url: Option<String>,
        tier: String,
        total_xp: i64,
        current_level: i32,
        current_streak: i32,
        unlocked_badges_bitmap: Vec<u8>,
    }
    let sql = if weekly_only {
        r#"SELECT u.id AS user_id, u.username, u.full_name, u.avatar_url, u.tier,
                  COALESCE(SUM(l.final_xp), gp.total_xp)::bigint AS total_xp,
                  gp.current_level, gp.current_streak, gp.unlocked_badges_bitmap
           FROM gamification_profiles gp
           JOIN users u ON u.id = gp.user_id
           LEFT JOIN xp_ledger_transactions l ON l.user_id = gp.user_id AND l.created_at >= NOW() - INTERVAL '7 days'
           GROUP BY u.id, u.username, u.full_name, u.avatar_url, u.tier, gp.total_xp, gp.current_level, gp.current_streak, gp.unlocked_badges_bitmap
           ORDER BY total_xp DESC, gp.current_level DESC LIMIT $1"#
    } else {
        r#"SELECT u.id AS user_id, u.username, u.full_name, u.avatar_url, u.tier,
                  gp.total_xp, gp.current_level, gp.current_streak, gp.unlocked_badges_bitmap
           FROM gamification_profiles gp
           JOIN users u ON u.id = gp.user_id
           ORDER BY gp.total_xp DESC, gp.current_level DESC LIMIT $1"#
    };
    let rows = sqlx::query_as::<_, RawLeaderRow>(sql).bind(limit).fetch_all(pool).await?;
    Ok(rows.into_iter().enumerate().map(|(idx, r)| {
        let bmap = decode_badges_bitmap(&r.unlocked_badges_bitmap);
        LeaderboardRow {
            rank: (idx + 1) as i64,
            user_id: r.user_id,
            username: r.username,
            full_name: r.full_name,
            avatar_url: r.avatar_url.unwrap_or_else(|| "assets/images/characters/avatar_1.png".to_string()),
            tier: r.tier,
            total_xp: r.total_xp,
            current_level: r.current_level,
            level_title: level_title_for(r.current_level as u32).to_string(),
            current_streak: r.current_streak,
            badges_count: bmap.len() as i32,
        }
    }).collect())
}
