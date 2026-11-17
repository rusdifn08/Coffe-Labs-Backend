use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::barista_toolkit::{
    error::BaristaToolkitError,
    models::{LmsCourseRow, LmsLessonRow, UserLmsProgressRow},
};

pub async fn fetch_published_courses(pool: &PgPool) -> Result<Vec<LmsCourseRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, LmsCourseRow>(
        r#"SELECT id, slug, title, subtitle, instructor_name, difficulty::text AS difficulty,
                  required_tier, thumbnail_url, linked_ebook_id, competency_level,
                  focus_summary, interactive_engine_type, total_modules_count,
                  total_duration_sec, xp_reward_on_complete
           FROM lms_courses
           WHERE is_published = TRUE
           ORDER BY display_order ASC, created_at ASC"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_all_lessons(pool: &PgPool) -> Result<Vec<LmsLessonRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, LmsLessonRow>(
        r#"SELECT id, course_id, module_number, lesson_number, subchapter_code, title,
                  description, kind::text AS kind, hls_manifest_path, duration_sec,
                  quiz_questions_json, key_takeaways_markdown, chapter_markers_json,
                  formulas_latex_json, storyboard_scenes_json, simulation_config_json,
                  linked_ebook_code, linked_ebook_chapter, xp_reward
           FROM lms_lessons
           ORDER BY course_id, module_number ASC, lesson_number ASC"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_user_lms_progress(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserLmsProgressRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, UserLmsProgressRow>(
        r#"SELECT lesson_id, watched_segments_bitmap, last_position_sec,
                  completion_pct, quiz_score_pct, is_completed
           FROM user_lms_lesson_progress
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_lesson_and_course_tier(
    pool: &PgPool,
    lesson_id: Uuid,
) -> Result<(LmsLessonRow, String), BaristaToolkitError> {
    let lesson = sqlx::query_as::<_, LmsLessonRow>(
        r#"SELECT id, course_id, module_number, lesson_number, subchapter_code, title,
                  description, kind::text AS kind, hls_manifest_path, duration_sec,
                  quiz_questions_json, key_takeaways_markdown, chapter_markers_json,
                  formulas_latex_json, storyboard_scenes_json, simulation_config_json,
                  linked_ebook_code, linked_ebook_chapter, xp_reward
           FROM lms_lessons WHERE id = $1"#,
    )
    .bind(lesson_id)
    .fetch_optional(pool)
    .await?
    .ok_or(BaristaToolkitError::NotFound)?;

    let required_tier: String = sqlx::query_scalar(
        "SELECT required_tier FROM lms_courses WHERE id = $1",
    )
    .bind(lesson.course_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "Trial".to_string());

    Ok((lesson, required_tier))
}

pub async fn fetch_single_lesson_progress(
    pool: &PgPool,
    user_id: Uuid,
    lesson_id: Uuid,
) -> Result<Option<UserLmsProgressRow>, BaristaToolkitError> {
    let row = sqlx::query_as::<_, UserLmsProgressRow>(
        r#"SELECT lesson_id, watched_segments_bitmap, last_position_sec,
                  completion_pct, quiz_score_pct, is_completed
           FROM user_lms_lesson_progress
           WHERE user_id = $1 AND lesson_id = $2"#,
    )
    .bind(user_id)
    .bind(lesson_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_lesson_progress(
    pool: &PgPool,
    user_id: Uuid,
    lesson_id: Uuid,
    course_id: Uuid,
    bitmap: &[u8],
    last_position_sec: i32,
    completion_pct: i16,
    quiz_score_pct: Option<i16>,
    is_completed: bool,
    award_xp: i32,
) -> Result<(), BaristaToolkitError> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"INSERT INTO user_lms_lesson_progress (
               user_id, lesson_id, course_id, watched_segments_bitmap, last_position_sec,
               completion_pct, quiz_score_pct, is_completed, completed_at, updated_at
           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8, CASE WHEN $8 THEN NOW() ELSE NULL END, NOW())
           ON CONFLICT (user_id, lesson_id) DO UPDATE SET
               watched_segments_bitmap = EXCLUDED.watched_segments_bitmap,
               last_position_sec = GREATEST(user_lms_lesson_progress.last_position_sec, EXCLUDED.last_position_sec),
               completion_pct = GREATEST(user_lms_lesson_progress.completion_pct, EXCLUDED.completion_pct),
               quiz_score_pct = COALESCE(EXCLUDED.quiz_score_pct, user_lms_lesson_progress.quiz_score_pct),
               is_completed = user_lms_lesson_progress.is_completed OR EXCLUDED.is_completed,
               completed_at = COALESCE(user_lms_lesson_progress.completed_at, CASE WHEN EXCLUDED.is_completed THEN NOW() ELSE NULL END),
               updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(lesson_id)
    .bind(course_id)
    .bind(bitmap)
    .bind(last_position_sec)
    .bind(completion_pct)
    .bind(quiz_score_pct)
    .bind(is_completed)
    .execute(&mut *tx)
    .await?;

    if award_xp > 0 {
        let _ = sqlx::query(
            r#"INSERT INTO gamification_profiles (user_id, total_xp)
               VALUES ($1, $2)
               ON CONFLICT (user_id) DO UPDATE SET
                   total_xp = gamification_profiles.total_xp + EXCLUDED.total_xp,
                   updated_at = NOW()"#,
        )
        .bind(user_id)
        .bind(award_xp as i64)
        .execute(&mut *tx)
        .await;
    }

    tx.commit().await?;
    Ok(())
}
