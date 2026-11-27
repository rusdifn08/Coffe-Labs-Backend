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
        extractors::OptionalAuthUser,
        redis::{redis_get_bounded, redis_set_ex_bounded},
        AppState,
    },
    modules::barista_toolkit::{
        cloudinary_signer::{generate_optimized_image_url, resolve_protected_book_asset_url},
        dto::{EbookAssetFormatQuery, EbookChapterAstDto, EbookChaptersBundleDto},
        error::BaristaToolkitError,
        handlers::ebook_handler::ensure_optional_ebook_access,
        repositories::{ebook_ast_repo, ebook_repo},
        services::object_storage_stream::stream_object_file,
    },
};

pub async fn get_ebook_chapters_ast(
    State(state): State<AppState>,
    OptionalAuthUser(opt_claims): OptionalAuthUser,
    Path(ebook_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let ebook = ebook_repo::fetch_single_ebook(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;
    ensure_optional_ebook_access(&state.pool, opt_claims.as_ref(), &ebook.required_tier).await?;

    let cache_key = format!("ebook:ast:v4:{ebook_id}");
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
    OptionalAuthUser(opt_claims): OptionalAuthUser,
    headers: HeaderMap,
    Path(ebook_id): Path<Uuid>,
    Query(query): Query<EbookAssetFormatQuery>,
) -> AppResult<impl IntoResponse> {
    let ebook = ebook_repo::fetch_single_ebook(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;
    ensure_optional_ebook_access(&state.pool, opt_claims.as_ref(), &ebook.required_tier).await?;

    let fmt = query.format.as_deref().unwrap_or("pdf").to_ascii_lowercase();
    let code = &ebook.book_code;
    let object_key = match fmt.as_str() {
        "pdf" => format!("coffeelabs/ebooks/{code}/{code}_id.pdf"),
        "epub" => format!("coffeelabs/ebooks/{code}/{code}_id.epub"),
        "chapters" | "ast" => format!("coffeelabs/ebooks/{code}/{code}_chapters.json"),
        _ => format!("coffeelabs/ebooks/{code}/{code}_ast.clbook"),
    };

    // Streaming hemat RAM (64 KB chunk + HTTP 206 Range Seeking) langsung dari Managed Object Storage
    if let Ok(resp) = stream_object_file(&headers, &object_key).await {
        return Ok(resp);
    }

    // Fallback ke direktori legacy `.clbook` bila format clbook belum ada di object_storage
    let legacy = format!("storage/ebooks_clbook/{code}.clbook");
    let bytes = tokio::fs::read(PathBuf::from(&legacy))
        .await
        .map_err(|_| AppError::from(BaristaToolkitError::NotFound))?;
    let mime = match fmt.as_str() {
        "pdf" => "application/pdf",
        "epub" => "application/epub+zip",
        "chapters" | "ast" => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    };
    axum::http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(format!("Response build error: {e}")))
}

/// Mengalirkan file media publik/terindeks (HLS .m3u8/.ts, MP4, WebVTT, Gambar, E-Book)
/// dengan dukungan HTTP 206 Partial Content Range & chunk streaming hemat RAM 512 MB.
pub async fn serve_object_storage_file(
    headers: HeaderMap,
    Path(raw_key): Path<String>,
) -> AppResult<impl IntoResponse> {
    stream_object_file(&headers, &raw_key).await
}
