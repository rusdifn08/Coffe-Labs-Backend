use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub use super::dto_game::*;
pub use super::dto_v3::*;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LmsLessonSummaryDto {
    pub id: Uuid,
    pub course_id: Uuid,
    pub module_number: i16,
    pub lesson_number: i16,
    pub subchapter_code: Option<String>,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub duration_sec: i32,
    pub completion_pct: i16,
    pub quiz_score_pct: Option<i16>,
    pub is_completed: bool,
    pub xp_reward: i32,
    pub key_takeaways_markdown: String,
    pub formulas_latex: serde_json::Value,
    pub storyboard_scenes: serde_json::Value,
    pub simulation_config: serde_json::Value,
    pub chapter_markers: serde_json::Value,
    pub quiz_questions: serde_json::Value,
    pub linked_ebook_code: Option<String>,
    pub linked_ebook_chapter: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LmsCourseDto {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub subtitle: String,
    pub instructor_name: String,
    pub difficulty: String,
    pub required_tier: String,
    pub is_unlocked: bool,
    pub thumbnail_url: String,
    pub competency_level: Option<String>,
    pub focus_summary: Option<String>,
    pub interactive_engine_type: Option<String>,
    pub linked_ebook_id: Option<Uuid>,
    pub total_modules_count: i16,
    pub total_duration_sec: i32,
    pub xp_reward_on_complete: i32,
    pub completed_lessons_count: usize,
    pub lessons: Vec<LmsLessonSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SignedHlsLessonResponse {
    pub lesson_id: Uuid,
    pub title: String,
    pub description: String,
    pub kind: String,
    pub signed_hls_url: String,
    pub expires_at: DateTime<Utc>,
    pub duration_sec: i32,
    pub key_takeaways_markdown: String,
    pub formulas_latex: serde_json::Value,
    pub storyboard_scenes: serde_json::Value,
    pub simulation_config: serde_json::Value,
    pub chapter_markers: serde_json::Value,
    pub quiz_questions: serde_json::Value,
    pub watched_bitmap_b64: String,
    pub last_position_sec: i32,
    pub completion_pct: i16,
    pub is_completed: bool,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct SyncLmsProgressRequest {
    #[validate(length(min = 1, max = 64))]
    pub watched_bitmap_b64: String,
    #[serde(default)]
    pub last_position_sec: i32,
    #[serde(default)]
    pub quiz_answers: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SyncLmsProgressResponse {
    pub lesson_id: Uuid,
    pub watched_bitmap_b64: String,
    pub completion_pct: i16,
    pub quiz_score_pct: Option<i16>,
    pub is_completed: bool,
    pub xp_awarded: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EbookCatalogItemDto {
    pub id: Uuid,
    pub book_code: String,
    pub title: String,
    pub original_title: Option<String>,
    pub author: String,
    pub synopsis: String,
    pub category: String,
    pub required_tier: String,
    pub is_unlocked: bool,
    pub cover_image_url: String,
    pub total_pages: i32,
    pub total_chapters_count: i16,
    pub total_figures_count: i16,
    pub supports_bilingual_toggle: bool,
    pub object_storage_provider: String,
    pub file_size_bytes: i64,
    pub pdf_file_size_bytes: i64,
    pub epub_file_size_bytes: i64,
    pub sha256_ciphertext_hash: String,
    pub toc_chapters: serde_json::Value,
    pub current_page: i32,
    pub current_chapter_number: i16,
    pub completion_pct: i16,
    pub is_completed: bool,
    pub bookmarks: serde_json::Value,
    pub highlights_notes: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct EbookLicenseRequest {
    #[validate(length(min = 3, max = 128))]
    pub device_installation_id: String,
    #[serde(default)]
    pub device_wrapping_secret_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EbookLicenseResponse {
    pub ebook_id: Uuid,
    pub book_code: String,
    pub wrapped_cek_b64: String,
    pub wrap_nonce_b64: String,
    pub aad_context: String,
    pub signed_asset_url: String,
    pub signed_pdf_url: String,
    pub signed_epub_url: String,
    pub sha256_ciphertext_hash: String,
    pub pdf_sha256_hash: Option<String>,
    pub epub_sha256_hash: Option<String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct SyncEbookProgressRequest {
    #[validate(range(min = 1, max = 5000))]
    pub current_page: i32,
    #[serde(default)]
    pub current_chapter_number: Option<i16>,
    #[serde(default)]
    pub highest_page_reached: Option<i32>,
    #[serde(default)]
    pub read_pages: Option<Vec<u32>>,
    #[serde(default)]
    pub bookmarks_json: Option<serde_json::Value>,
    #[serde(default)]
    pub highlights_notes_json: Option<serde_json::Value>,
    #[serde(default)]
    pub preferred_reader_mode: Option<String>,
    #[serde(default)]
    pub epub_cfi_locator: Option<String>,
}
