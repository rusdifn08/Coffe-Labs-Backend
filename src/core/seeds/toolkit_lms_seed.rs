use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;

const LMS_SEED_JSON: &str = include_str!("../../../storage/lms_manual_brew_seed.json");

#[derive(Deserialize)]
struct LmsBundleSeed {
    courses: Vec<CourseSeed>,
}

#[derive(Deserialize)]
struct CourseSeed {
    id: Uuid,
    slug: String,
    title: String,
    subtitle: String,
    instructor_name: String,
    difficulty: String,
    required_tier: String,
    competency_level: Option<String>,
    focus_summary: Option<String>,
    interactive_engine_type: Option<String>,
    linked_ebook_code: Option<String>,
    total_modules_count: i16,
    total_duration_sec: i32,
    xp_reward_on_complete: i32,
    display_order: i16,
    lessons: Vec<LessonSeed>,
}

#[derive(Deserialize)]
struct LessonSeed {
    id: Uuid,
    #[allow(dead_code)]
    course_id: Uuid,
    module_number: i16,
    lesson_number: i16,
    subchapter_code: Option<String>,
    title: String,
    description: String,
    kind: String,
    hls_manifest_path: String,
    duration_sec: i32,
    xp_reward: i32,
    linked_ebook_code: Option<String>,
    linked_ebook_chapter: Option<i16>,
    key_takeaways_markdown: String,
    formulas_latex_json: serde_json::Value,
    storyboard_scenes_json: serde_json::Value,
    simulation_config_json: serde_json::Value,
    chapter_markers_json: serde_json::Value,
    quiz_questions_json: serde_json::Value,
}

pub async fn seed_lms_courses_and_lessons(pool: &PgPool) -> AppResult<()> {
    let Ok(bundle) = serde_json::from_str::<LmsBundleSeed>(LMS_SEED_JSON) else {
        return Ok(());
    };

    for c in bundle.courses {
        let code = c.linked_ebook_code.as_deref().unwrap_or("CL-BOOK-03");
        let cid: Uuid = sqlx::query_scalar(
            r#"INSERT INTO lms_courses (
                   id, slug, title, subtitle, instructor_name, difficulty, required_tier,
                   thumbnail_url, competency_level, focus_summary, interactive_engine_type,
                   total_modules_count, total_duration_sec, xp_reward_on_complete,
                   is_published, display_order
               ) VALUES ($1,$2,$3,$4,$5,$6::course_difficulty_level,$7,$8,$9,$10,$11,$12,$13,$14,TRUE,$15)
               ON CONFLICT (slug) DO UPDATE SET
                   title = EXCLUDED.title, subtitle = EXCLUDED.subtitle,
                   competency_level = EXCLUDED.competency_level,
                   focus_summary = EXCLUDED.focus_summary,
                   interactive_engine_type = EXCLUDED.interactive_engine_type,
                   total_modules_count = EXCLUDED.total_modules_count,
                   total_duration_sec = EXCLUDED.total_duration_sec,
                   is_published = TRUE
               RETURNING id"#,
        )
        .bind(c.id).bind(&c.slug).bind(&c.title).bind(&c.subtitle)
        .bind(&c.instructor_name).bind(&c.difficulty).bind(&c.required_tier)
        .bind(format!("/api/v1/toolkit/object-storage/coffeelabs/ebooks/{code}/cover.webp"))
        .bind(&c.competency_level).bind(&c.focus_summary).bind(&c.interactive_engine_type)
        .bind(c.total_modules_count).bind(c.total_duration_sec)
        .bind(c.xp_reward_on_complete).bind(c.display_order)
        .fetch_one(pool)
        .await?;

        for l in c.lessons {
            sqlx::query(
                r#"INSERT INTO lms_lessons (
                       id, course_id, module_number, lesson_number, subchapter_code, title,
                       description, kind, hls_manifest_path, duration_sec,
                       quiz_questions_json, key_takeaways_markdown, chapter_markers_json,
                       formulas_latex_json, storyboard_scenes_json, simulation_config_json,
                       linked_ebook_code, linked_ebook_chapter, xp_reward
                   ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8::lms_lesson_kind,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
                   ON CONFLICT (course_id, module_number, lesson_number) DO UPDATE SET
                       subchapter_code = EXCLUDED.subchapter_code,
                       title = EXCLUDED.title, description = EXCLUDED.description,
                       key_takeaways_markdown = EXCLUDED.key_takeaways_markdown,
                       formulas_latex_json = EXCLUDED.formulas_latex_json,
                       storyboard_scenes_json = EXCLUDED.storyboard_scenes_json,
                       simulation_config_json = EXCLUDED.simulation_config_json,
                       quiz_questions_json = EXCLUDED.quiz_questions_json,
                       linked_ebook_code = EXCLUDED.linked_ebook_code,
                       linked_ebook_chapter = EXCLUDED.linked_ebook_chapter"#,
            )
            .bind(l.id).bind(cid).bind(l.module_number).bind(l.lesson_number)
            .bind(&l.subchapter_code).bind(&l.title).bind(&l.description).bind(&l.kind)
            .bind(&l.hls_manifest_path).bind(l.duration_sec).bind(&l.quiz_questions_json)
            .bind(&l.key_takeaways_markdown).bind(&l.chapter_markers_json)
            .bind(&l.formulas_latex_json).bind(&l.storyboard_scenes_json)
            .bind(&l.simulation_config_json).bind(&l.linked_ebook_code)
            .bind(l.linked_ebook_chapter).bind(l.xp_reward)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}
