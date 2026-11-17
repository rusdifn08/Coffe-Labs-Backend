use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EbookChapterAstDto {
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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EbookChaptersBundleDto {
    pub ebook_id: Uuid,
    pub book_code: String,
    pub title: String,
    pub author: String,
    pub supports_bilingual_toggle: bool,
    pub cover_url: String,
    pub signed_pdf_url: String,
    pub signed_epub_url: String,
    pub signed_clbook_url: String,
    pub chapters: Vec<EbookChapterAstDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CoffeeDictionaryEntryDto {
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

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DictionaryFilterQuery {
    pub letter: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct EbookAssetFormatQuery {
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LmsCompletionEvent {
    pub user_id: Uuid,
    pub lesson_id: Uuid,
    pub course_id: Uuid,
    pub xp_awarded: i32,
    pub completion_pct: i16,
    pub quiz_score_pct: Option<i16>,
    pub certificate_public_id: String,
}
