use sqlx::PgPool;
use tracing::warn;

use crate::modules::users::cloud_state_dto::UserCloudStateResponse;

/// Menyinkronkan progres EXP, level, streak, nama, dan avatar ke `gamification_profiles`
/// serta `user_profile_projections` agar Leaderboard dan Social Profile selalu seragam.
pub async fn sync_gamification_and_projections(pool: &PgPool, state: &UserCloudStateResponse) {
    let gp_res = sqlx::query(
        r#"INSERT INTO gamification_profiles (user_id, total_xp, current_level, current_streak, longest_streak, updated_at)
           VALUES ($1, $2, $3, $4, $4, NOW())
           ON CONFLICT (user_id) DO UPDATE SET
               total_xp = GREATEST(gamification_profiles.total_xp, EXCLUDED.total_xp),
               current_level = GREATEST(gamification_profiles.current_level, EXCLUDED.current_level),
               current_streak = GREATEST(gamification_profiles.current_streak, EXCLUDED.current_streak),
               longest_streak = GREATEST(gamification_profiles.longest_streak, EXCLUDED.current_streak),
               updated_at = NOW()"#,
    )
    .bind(state.user_id)
    .bind(state.total_exp as i64)
    .bind(state.current_level)
    .bind(state.streak_days)
    .execute(pool)
    .await;
    if let Err(err) = gp_res {
        warn!(user_id = %state.user_id, error = %err, "Gagal sync gamification_profiles dari cloud_state");
    }

    let proj_res = sqlx::query(
        r#"UPDATE user_profile_projections
           SET display_name = $2,
               avatar_url = $3,
               total_xp = GREATEST(total_xp, $4),
               barista_level = GREATEST(barista_level, $5),
               barista_rank_title = $6,
               current_streak_days = GREATEST(current_streak_days, $7),
               total_brews_count = GREATEST(total_brews_count, $8),
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(state.user_id)
    .bind(&state.full_name)
    .bind(&state.avatar_url)
    .bind(state.total_exp as i64)
    .bind(state.current_level)
    .bind(&state.level_title)
    .bind(state.streak_days)
    .bind(state.brews_count)
    .execute(pool)
    .await;
    if let Err(err) = proj_res {
        warn!(user_id = %state.user_id, error = %err, "Gagal sync user_profile_projections dari cloud_state");
    }
}
