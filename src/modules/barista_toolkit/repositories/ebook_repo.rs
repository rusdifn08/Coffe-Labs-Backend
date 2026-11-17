use std::io::Cursor;

use roaring::RoaringBitmap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::barista_toolkit::{
    error::BaristaToolkitError,
    models::{ToolkitEbookRow, UserEbookProgressRow},
};

pub async fn fetch_ebooks_catalog(pool: &PgPool) -> Result<Vec<ToolkitEbookRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, ToolkitEbookRow>(
        r#"SELECT id, book_code, title, original_title, author, synopsis, cover_image_url,
                  cover_cloudinary_public_id, pdf_cloudinary_public_id, epub_cloudinary_public_id,
                  clbook_ast_cloudinary_public_id, category, required_tier, total_pages,
                  total_chapters_count, total_figures_count, supports_bilingual_toggle,
                  object_storage_provider, file_size_bytes, pdf_file_size_bytes, epub_file_size_bytes,
                  sha256_ciphertext_hash, pdf_sha256_hash, epub_sha256_hash,
                  wrapped_master_cek, toc_chapters_json, xp_reward_on_finish
           FROM toolkit_ebooks
           ORDER BY book_code ASC"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_single_ebook(
    pool: &PgPool,
    ebook_id: Uuid,
) -> Result<ToolkitEbookRow, BaristaToolkitError> {
    let row = sqlx::query_as::<_, ToolkitEbookRow>(
        r#"SELECT id, book_code, title, original_title, author, synopsis, cover_image_url,
                  cover_cloudinary_public_id, pdf_cloudinary_public_id, epub_cloudinary_public_id,
                  clbook_ast_cloudinary_public_id, category, required_tier, total_pages,
                  total_chapters_count, total_figures_count, supports_bilingual_toggle,
                  object_storage_provider, file_size_bytes, pdf_file_size_bytes, epub_file_size_bytes,
                  sha256_ciphertext_hash, pdf_sha256_hash, epub_sha256_hash,
                  wrapped_master_cek, toc_chapters_json, xp_reward_on_finish
           FROM toolkit_ebooks WHERE id = $1"#,
    )
    .bind(ebook_id)
    .fetch_optional(pool)
    .await?
    .ok_or(BaristaToolkitError::NotFound)?;
    Ok(row)
}

pub async fn fetch_user_ebook_progress_map(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserEbookProgressRow>, BaristaToolkitError> {
    let rows = sqlx::query_as::<_, UserEbookProgressRow>(
        r#"SELECT ebook_id, current_page, current_chapter_number, highest_page_reached,
                  read_pages_bitmap, completion_pct, bookmarks_json, highlights_notes_json,
                  is_completed
           FROM user_ebook_progress
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_ebook_progress(
    pool: &PgPool,
    user_id: Uuid,
    ebook_id: Uuid,
    total_pages: i32,
    current_page: i32,
    current_chapter_number: i16,
    highest_page: i32,
    new_read_pages: &[u32],
    bookmarks: Option<&serde_json::Value>,
    highlights: Option<&serde_json::Value>,
    preferred_mode: Option<&str>,
    epub_cfi: Option<&str>,
) -> Result<(i16, bool), BaristaToolkitError> {
    let prev = sqlx::query_as::<_, UserEbookProgressRow>(
        r#"SELECT ebook_id, current_page, current_chapter_number, highest_page_reached,
                  read_pages_bitmap, completion_pct, bookmarks_json, highlights_notes_json,
                  is_completed
           FROM user_ebook_progress WHERE user_id = $1 AND ebook_id = $2"#,
    )
    .bind(user_id)
    .bind(ebook_id)
    .fetch_optional(pool)
    .await?;

    let mut bm = prev
        .as_ref()
        .and_then(|p| p.read_pages_bitmap.as_deref())
        .and_then(|bytes| RoaringBitmap::deserialize_from(Cursor::new(bytes)).ok())
        .unwrap_or_default();
    bm.insert(current_page.max(1) as u32);
    for &p in new_read_pages {
        if p >= 1 && p <= total_pages.max(1) as u32 {
            bm.insert(p);
        }
    }

    let mut bm_bytes = Vec::with_capacity(64);
    let _ = bm.serialize_into(&mut bm_bytes);

    let effective_pages = total_pages.max(1) as f64;
    let completion_pct = ((bm.len() as f64 / effective_pages) * 100.0)
        .round()
        .clamp(0.0, 100.0) as i16;
    let is_completed = completion_pct >= 80 || highest_page >= total_pages;

    let merged_bookmarks = bookmarks
        .cloned()
        .or_else(|| prev.as_ref().map(|p| p.bookmarks_json.clone()))
        .unwrap_or_else(|| serde_json::json!([]));
    let merged_highlights = highlights
        .cloned()
        .or_else(|| prev.as_ref().map(|p| p.highlights_notes_json.clone()))
        .unwrap_or_else(|| serde_json::json!([]));

    sqlx::query(
        r#"INSERT INTO user_ebook_progress (
               user_id, ebook_id, current_page, current_chapter_number, highest_page_reached,
               read_pages_bitmap, completion_pct, bookmarks_json, highlights_notes_json,
               preferred_reader_mode, epub_cfi_locator, is_completed, last_read_at
           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NOW())
           ON CONFLICT (user_id, ebook_id) DO UPDATE SET
               current_page = EXCLUDED.current_page,
               current_chapter_number = EXCLUDED.current_chapter_number,
               highest_page_reached = GREATEST(user_ebook_progress.highest_page_reached, EXCLUDED.highest_page_reached),
               read_pages_bitmap = EXCLUDED.read_pages_bitmap,
               completion_pct = GREATEST(user_ebook_progress.completion_pct, EXCLUDED.completion_pct),
               bookmarks_json = EXCLUDED.bookmarks_json,
               highlights_notes_json = EXCLUDED.highlights_notes_json,
               preferred_reader_mode = EXCLUDED.preferred_reader_mode,
               epub_cfi_locator = COALESCE(EXCLUDED.epub_cfi_locator, user_ebook_progress.epub_cfi_locator),
               is_completed = user_ebook_progress.is_completed OR EXCLUDED.is_completed,
               last_read_at = NOW()"#,
    )
    .bind(user_id)
    .bind(ebook_id)
    .bind(current_page)
    .bind(current_chapter_number)
    .bind(highest_page)
    .bind(bm_bytes)
    .bind(completion_pct)
    .bind(merged_bookmarks)
    .bind(merged_highlights)
    .bind(preferred_mode.unwrap_or("MOBILE_AST"))
    .bind(epub_cfi)
    .bind(is_completed)
    .execute(pool)
    .await?;

    Ok((completion_pct, is_completed))
}
