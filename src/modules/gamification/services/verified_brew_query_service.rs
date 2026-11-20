use std::collections::HashMap;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::gamification::{
        level_curve_math::compute_level_snapshot,
        verified_brew_dto::{GamificationSummaryDto, TriedRecipeDto},
    },
};

pub async fn get_me_summary(pool: &PgPool, user_id: Uuid) -> AppResult<GamificationSummaryDto> {
    let prof = sqlx::query(
        r#"
        SELECT total_xp, total_verified_brews_count, current_streak_days
        FROM user_gamification_profiles WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let (total_xp, brews_count, streak) = match prof {
        Some(r) => (
            r.try_get::<i64, _>("total_xp")?,
            r.try_get::<i32, _>("total_verified_brews_count")?,
            r.try_get::<i32, _>("current_streak_days")?,
        ),
        None => (0, 0, 0),
    };
    let snap = compute_level_snapshot(total_xp);

    let method_rows = sqlx::query(
        "SELECT method_id, completed_count FROM user_method_brew_stats WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut method_completed_counts = HashMap::new();
    for r in method_rows {
        method_completed_counts.insert(r.try_get("method_id")?, r.try_get("completed_count")?);
    }

    let tried_rows = sqlx::query(
        r#"
        SELECT recipe_id, recipe_title, method_id, times_completed, best_accuracy_pct, last_duration_sec, last_completed_at
        FROM user_tried_recipes WHERE user_id = $1 ORDER BY last_completed_at DESC LIMIT 20
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut recently_tried_recipes = Vec::with_capacity(tried_rows.len());
    for r in tried_rows {
        recently_tried_recipes.push(TriedRecipeDto {
            recipe_id: r.try_get("recipe_id")?,
            recipe_title: r.try_get("recipe_title")?,
            method_id: r.try_get("method_id")?,
            times_completed: r.try_get("times_completed")?,
            best_accuracy_pct: r.try_get("best_accuracy_pct")?,
            last_duration_sec: r.try_get("last_duration_sec")?,
            last_completed_at: r.try_get("last_completed_at")?,
        });
    }

    Ok(GamificationSummaryDto {
        total_xp,
        current_level: snap.level,
        barista_rank_title: snap.rank_title.into(),
        level_start_xp: snap.level_start_xp,
        next_level_xp: snap.next_level_xp,
        level_progress_ratio: snap.progress_ratio,
        total_verified_brews_count: brews_count,
        current_streak_days: streak,
        method_completed_counts,
        recently_tried_recipes,
    })
}
