use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct LmsCourseRow {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub subtitle: String,
    pub instructor_name: String,
    pub difficulty: String,
    pub required_tier: String,
    pub thumbnail_url: String,
    pub linked_ebook_id: Option<Uuid>,
    pub competency_level: Option<String>,
    pub focus_summary: Option<String>,
    pub interactive_engine_type: Option<String>,
    pub total_modules_count: i16,
    pub total_duration_sec: i32,
    pub xp_reward_on_complete: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct LmsLessonRow {
    pub id: Uuid,
    pub course_id: Uuid,
    pub module_number: i16,
    pub lesson_number: i16,
    pub subchapter_code: Option<String>,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub hls_manifest_path: Option<String>,
    pub duration_sec: i32,
    pub quiz_questions_json: serde_json::Value,
    pub key_takeaways_markdown: Option<String>,
    pub chapter_markers_json: serde_json::Value,
    pub formulas_latex_json: serde_json::Value,
    pub storyboard_scenes_json: serde_json::Value,
    pub simulation_config_json: serde_json::Value,
    pub linked_ebook_code: Option<String>,
    pub linked_ebook_chapter: Option<i16>,
    pub xp_reward: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserLmsProgressRow {
    pub lesson_id: Uuid,
    pub watched_segments_bitmap: Vec<u8>,
    pub last_position_sec: i32,
    pub completion_pct: i16,
    pub quiz_score_pct: Option<i16>,
    pub is_completed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct ToolkitEbookRow {
    pub id: Uuid,
    pub book_code: String,
    pub title: String,
    pub original_title: Option<String>,
    pub author: String,
    pub synopsis: String,
    pub cover_image_url: String,
    pub cover_cloudinary_public_id: Option<String>,
    pub pdf_cloudinary_public_id: Option<String>,
    pub epub_cloudinary_public_id: Option<String>,
    pub clbook_ast_cloudinary_public_id: Option<String>,
    pub category: String,
    pub required_tier: String,
    pub total_pages: i32,
    pub total_chapters_count: i16,
    pub total_figures_count: i16,
    pub supports_bilingual_toggle: bool,
    pub object_storage_provider: String,
    pub file_size_bytes: i64,
    pub pdf_file_size_bytes: i64,
    pub epub_file_size_bytes: i64,
    pub sha256_ciphertext_hash: String,
    pub pdf_sha256_hash: Option<String>,
    pub epub_sha256_hash: Option<String>,
    pub wrapped_master_cek: Vec<u8>,
    pub toc_chapters_json: serde_json::Value,
    pub xp_reward_on_finish: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct EbookChapterAstRow {
    pub id: Uuid,
    pub chapter_number: i16,
    pub title_id: String,
    pub title_en: Option<String>,
    pub estimated_read_time_min: i16,
    pub start_pdf_page: i32,
    pub end_pdf_page: i32,
    pub mobile_blocks_ast: serde_json::Value,
    pub key_summary_points: serde_json::Value,
    pub chapter_quiz_json: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct CoffeeDictionaryRow {
    pub id: Uuid,
    pub term_slug: String,
    pub alphabet_letter: String,
    pub term_title: String,
    pub category: String,
    pub short_definition_id: String,
    pub deep_explanation_id: String,
    pub related_formula_latex: Option<String>,
    pub illustration_cloudinary_url: Option<String>,
    pub related_book_code: String,
    pub related_chapter_num: i16,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserEbookProgressRow {
    pub ebook_id: Uuid,
    pub current_page: i32,
    pub current_chapter_number: i16,
    pub highest_page_reached: i32,
    pub read_pages_bitmap: Option<Vec<u8>>,
    pub completion_pct: i16,
    pub bookmarks_json: serde_json::Value,
    pub highlights_notes_json: serde_json::Value,
    pub is_completed: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct GameBrewChapterRow {
    pub chapter_number: i16,
    pub slug: String,
    pub title: String,
    pub story_location: String,
    pub npc_mentor_name: String,
    pub target_recipe_slug: String,
    pub physics_config_json: serde_json::Value,
    pub story_dialogue_json: serde_json::Value,
    pub xp_reward_3_stars: i32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct UserGameBrewProgressRow {
    pub chapter_number: i16,
    pub stars_earned: i16,
    pub high_score_accuracy_pct: f32,
    pub best_extraction_yield_pct: Option<f32>,
    pub is_unlocked: bool,
    pub completed_at: Option<DateTime<Utc>>,
}

pub fn tier_level(tier: &str, role: &str) -> u8 {
    if role.eq_ignore_ascii_case("admin") {
        return 3;
    }
    match tier.trim().to_ascii_lowercase().as_str() {
        "business" => 3,
        "pro" => 2,
        _ => 1,
    }
}

pub fn has_tier_access(user_tier: &str, user_role: &str, required_tier: &str) -> bool {
    tier_level(user_tier, user_role) >= tier_level(required_tier, "user")
}
