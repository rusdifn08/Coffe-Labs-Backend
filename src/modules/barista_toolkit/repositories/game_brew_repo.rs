use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::barista_toolkit::{
    error::BaristaToolkitError,
    models::{GameBrewChapterRow, UserGameBrewProgressRow},
};

pub async fn fetch_all_chapters(pool: &PgPool) -> Result<Vec<GameBrewChapterRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, GameBrewChapterRow>(
        r#"SELECT chapter_number, slug, title, story_location, npc_mentor_name,
                  target_recipe_slug, physics_config_json, story_dialogue_json, xp_reward_3_stars
           FROM game_brew_chapters
           ORDER BY chapter_number ASC"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_single_chapter(
    pool: &PgPool,
    chapter_number: i16,
) -> Result<GameBrewChapterRow, BaristaToolkitError> {
    let row = sqlx::query_as::<_, GameBrewChapterRow>(
        r#"SELECT chapter_number, slug, title, story_location, npc_mentor_name,
                  target_recipe_slug, physics_config_json, story_dialogue_json, xp_reward_3_stars
           FROM game_brew_chapters WHERE chapter_number = $1"#,
    )
    .bind(chapter_number)
    .fetch_optional(pool)
    .await?
    .ok_or(BaristaToolkitError::NotFound)?;
    Ok(row)
}

pub async fn fetch_user_game_progress(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserGameBrewProgressRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, UserGameBrewProgressRow>(
        r#"SELECT chapter_number, stars_earned, high_score_accuracy_pct,
                  best_extraction_yield_pct, is_unlocked, completed_at
           FROM user_game_brew_progress
           WHERE user_id = $1
           ORDER BY chapter_number ASC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
pub async fn save_chapter_submission(
    pool: &PgPool,
    user_id: Uuid,
    chapter_number: i16,
    stars_earned: i16,
    accuracy_pct: f32,
    extraction_yield_est: f32,
    unlocked_next: Option<i16>,
    xp_awarded: i32,
) -> Result<(), BaristaToolkitError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"INSERT INTO user_game_brew_progress (
               user_id, chapter_number, stars_earned, high_score_accuracy_pct,
               best_extraction_yield_pct, attempts_count, is_unlocked, completed_at, updated_at
           ) VALUES ($1,$2,$3,$4,$5,1,TRUE, CASE WHEN $3 > 0 THEN NOW() ELSE NULL END, NOW())
           ON CONFLICT (user_id, chapter_number) DO UPDATE SET
               stars_earned = GREATEST(user_game_brew_progress.stars_earned, EXCLUDED.stars_earned),
               high_score_accuracy_pct = GREATEST(user_game_brew_progress.high_score_accuracy_pct, EXCLUDED.high_score_accuracy_pct),
               best_extraction_yield_pct = COALESCE(EXCLUDED.best_extraction_yield_pct, user_game_brew_progress.best_extraction_yield_pct),
               attempts_count = user_game_brew_progress.attempts_count + 1,
               is_unlocked = TRUE,
               completed_at = COALESCE(user_game_brew_progress.completed_at, EXCLUDED.completed_at),
               updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(chapter_number)
    .bind(stars_earned)
    .bind(accuracy_pct)
    .bind(extraction_yield_est)
    .execute(&mut *tx)
    .await?;

    if let Some(next_ch) = unlocked_next {
        sqlx::query(
            r#"INSERT INTO user_game_brew_progress (user_id, chapter_number, is_unlocked)
               VALUES ($1, $2, TRUE)
               ON CONFLICT (user_id, chapter_number) DO UPDATE SET is_unlocked = TRUE"#,
        )
        .bind(user_id)
        .bind(next_ch)
        .execute(&mut *tx)
        .await?;
    }

    if xp_awarded > 0 {
        let _ = sqlx::query(
            r#"INSERT INTO gamification_profiles (user_id, total_xp)
               VALUES ($1, $2)
               ON CONFLICT (user_id) DO UPDATE SET
                   total_xp = gamification_profiles.total_xp + EXCLUDED.total_xp,
                   updated_at = NOW()"#,
        )
        .bind(user_id)
        .bind(xp_awarded as i64)
        .execute(&mut *tx)
        .await;
    }

    tx.commit().await?;
    Ok(())
}
