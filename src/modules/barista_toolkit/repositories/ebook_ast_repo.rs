use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::barista_toolkit::{
    error::BaristaToolkitError,
    models::{CoffeeDictionaryRow, EbookChapterAstRow},
};

pub async fn fetch_ebook_chapters_ast(
    pool: &PgPool,
    ebook_id: Uuid,
) -> Result<Vec<EbookChapterAstRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, EbookChapterAstRow>(
        r#"SELECT id, chapter_number, title_id, title_en, estimated_read_time_min,
                  start_pdf_page, end_pdf_page, mobile_blocks_ast,
                  key_summary_points, chapter_quiz_json
           FROM ebook_chapters_ast
           WHERE ebook_id = $1
           ORDER BY chapter_number ASC"#,
    )
    .bind(ebook_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_dictionary_entries(
    pool: &PgPool,
    letter: Option<&str>,
    query_str: Option<&str>,
) -> Result<Vec<CoffeeDictionaryRow>, BaristaToolkitError> {
    let l = letter
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().next().unwrap_or('A').to_ascii_uppercase().to_string());
    let q = query_str
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let rows = sqlx::query_as::<_, CoffeeDictionaryRow>(
        r#"SELECT id, term_slug, alphabet_letter, term_title, category,
                  short_definition_id, deep_explanation_id, related_formula_latex,
                  illustration_cloudinary_url, related_book_code, related_chapter_num
           FROM coffee_dictionary_entries
           WHERE ($1::text IS NULL OR UPPER(alphabet_letter) = $1)
             AND ($2::text IS NULL OR LOWER(term_title) LIKE $2 OR LOWER(short_definition_id) LIKE $2)
           ORDER BY term_title ASC"#,
    )
    .bind(l)
    .bind(q)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_dictionary_entry_by_slug(
    pool: &PgPool,
    slug: &str,
) -> Result<CoffeeDictionaryRow, BaristaToolkitError> {
    let row = sqlx::query_as::<_, CoffeeDictionaryRow>(
        r#"SELECT id, term_slug, alphabet_letter, term_title, category,
                  short_definition_id, deep_explanation_id, related_formula_latex,
                  illustration_cloudinary_url, related_book_code, related_chapter_num
           FROM coffee_dictionary_entries
           WHERE term_slug = $1"#,
    )
    .bind(slug.trim())
    .fetch_optional(pool)
    .await?
    .ok_or(BaristaToolkitError::NotFound)?;
    Ok(row)
}
