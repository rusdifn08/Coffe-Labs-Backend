use std::path::PathBuf;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::AuthUser,
        redis::{redis_get_bounded, redis_set_ex_bounded},
        AppState,
    },
    modules::barista_toolkit::{
        cloudinary_signer::{generate_optimized_image_url, resolve_protected_book_asset_url},
        dto::{EbookAssetFormatQuery, EbookChapterAstDto, EbookChaptersBundleDto},
        error::BaristaToolkitError,
        repositories::{ebook_ast_repo, ebook_repo},
        services::{
            object_storage_stream::stream_object_file,
            tier_access_resolver::user_has_tier_access,
        },
    },
};

pub async fn get_ebook_chapters_ast(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(ebook_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let ebook = ebook_repo::fetch_single_ebook(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;
    if !user_has_tier_access(&state.pool, &claims, &ebook.required_tier).await {
        return Err(AppError::from(BaristaToolkitError::TierUpgradeRequired(
            ebook.required_tier,
        )));
    }

    let cache_key = format!("ebook:ast:v3:{ebook_id}");
    if let Some(pool) = state.redis.active_pool() {
        if let Some(cached_json) = redis_get_bounded(&pool, &cache_key).await {
            if let Ok(bundle) = serde_json::from_str::<EbookChaptersBundleDto>(&cached_json) {
                return Ok(ApiResponse::ok("AST Bab E-Book dimuat dari Redis L2 cache.", bundle));
            }
        }
    }

    let rows = ebook_ast_repo::fetch_ebook_chapters_ast(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;

    let cloud_creds = state.config.cloudinary_config().ok().flatten();
    let is_cloud = ebook.object_storage_provider.eq_ignore_ascii_case("CLOUDINARY");
    let use_creds = if is_cloud { cloud_creds.as_ref() } else { None };

    let chapters: Vec<EbookChapterAstDto> = rows
        .into_iter()
        .map(|r| EbookChapterAstDto {
            id: r.id,
            chapter_number: r.chapter_number,
            title_id: r.title_id,
            title_en: r.title_en,
            estimated_read_time_min: r.estimated_read_time_min,
            start_pdf_page: r.start_pdf_page,
            end_pdf_page: r.end_pdf_page,
            mobile_blocks_ast: r.mobile_blocks_ast,
            key_summary_points: r.key_summary_points,
            chapter_quiz_json: r.chapter_quiz_json,
        })
        .collect();

    let cover_url = ebook
        .cover_cloudinary_public_id
        .as_deref()
        .map(|pid| generate_optimized_image_url(use_creds, pid, 1000))
        .unwrap_or(ebook.cover_image_url);

    let bundle = EbookChaptersBundleDto {
        ebook_id: ebook.id,
        book_code: ebook.book_code,
        title: ebook.title,
        author: ebook.author,
        supports_bilingual_toggle: ebook.supports_bilingual_toggle,
        cover_url,
        signed_pdf_url: resolve_protected_book_asset_url(
            use_creds, ebook.pdf_cloudinary_public_id.as_deref(),
            format!("/api/v1/toolkit/ebooks/{}/asset?format=pdf", ebook.id), 3600,
        ),
        signed_epub_url: resolve_protected_book_asset_url(
            use_creds, ebook.epub_cloudinary_public_id.as_deref(),
            format!("/api/v1/toolkit/ebooks/{}/asset?format=epub", ebook.id), 3600,
        ),
        signed_clbook_url: resolve_protected_book_asset_url(
            use_creds, ebook.clbook_ast_cloudinary_public_id.as_deref(),
            format!("/api/v1/toolkit/ebooks/{}/asset?format=clbook", ebook.id), 3600,
        ),
        chapters,
    };

    if let Some(pool) = state.redis.active_pool() {
        if let Ok(json_str) = serde_json::to_string(&bundle) {
            redis_set_ex_bounded(&pool, &cache_key, json_str, 600).await;
        }
    }

    Ok(ApiResponse::ok("AST Bab E-Book siap dirender di reader tri-mode.", bundle))
}

pub async fn download_encrypted_clbook(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(ebook_id): Path<Uuid>,
    Query(query): Query<EbookAssetFormatQuery>,
) -> AppResult<impl IntoResponse> {
    let ebook = ebook_repo::fetch_single_ebook(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;
    if !user_has_tier_access(&state.pool, &claims, &ebook.required_tier).await {
        return Err(AppError::from(BaristaToolkitError::TierUpgradeRequired(
            ebook.required_tier,
        )));
    }

    let fmt = query.format.as_deref().unwrap_or("clbook").to_ascii_lowercase();
    let code = &ebook.book_code;
    let (primary, mime) = match fmt.as_str() {
        "pdf" => (format!("storage/object_storage/coffeelabs/ebooks/{code}/{code}_id.pdf"), "application/pdf"),
        "epub" => (format!("storage/object_storage/coffeelabs/ebooks/{code}/{code}_id.epub"), "application/epub+zip"),
        _ => (format!("storage/object_storage/coffeelabs/ebooks/{code}/{code}_ast.clbook"), "application/octet-stream"),
    };
    let legacy = format!("storage/ebooks_clbook/{code}.clbook");
    let bytes = match tokio::fs::read(PathBuf::from(&primary)).await {
        Ok(b) => b,
        Err(_) => tokio::fs::read(PathBuf::from(&legacy))
            .await
            .map_err(|_| AppError::from(BaristaToolkitError::NotFound))?,
    };
    Ok((StatusCode::OK, [(header::CONTENT_TYPE, mime)], Body::from(bytes)))
}

/// Mengalirkan file media publik/terindeks (HLS .m3u8/.ts, MP4, WebVTT, Gambar, E-Book)
/// dengan dukungan HTTP 206 Partial Content Range & chunk streaming hemat RAM 512 MB.
pub async fn serve_object_storage_file(
    headers: HeaderMap,
    Path(raw_key): Path<String>,
) -> AppResult<impl IntoResponse> {
    stream_object_file(&headers, &raw_key).await
}
