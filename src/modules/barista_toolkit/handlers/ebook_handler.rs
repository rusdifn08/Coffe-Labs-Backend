use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::barista_toolkit::{
        cloudinary_signer::{generate_optimized_image_url, resolve_protected_book_asset_url},
        dto::{
            EbookCatalogItemDto, EbookLicenseRequest, EbookLicenseResponse,
            SyncEbookProgressRequest,
        },
        error::BaristaToolkitError,
        models::has_tier_access,
        repositories::ebook_repo,
        services::{
            drm_crypto_service::{resolved_drm_secret, unwrap_master_cek, wrap_cek_for_device},
            tier_access_resolver::{resolve_live_effective_tier, user_has_tier_access},
        },
    },
};

pub async fn list_ebooks(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let (loaded, (live_tier, live_role)) = tokio::join!(
        async {
            tokio::try_join!(
                ebook_repo::fetch_ebooks_catalog(&state.pool),
                ebook_repo::fetch_user_ebook_progress_map(&state.pool, claims.sub),
            )
            .map_err(AppError::from)
        },
        resolve_live_effective_tier(&state.pool, &claims),
    );
    let (books, progress_list) = loaded?;

    let cloud_creds = state.config.cloudinary_config().ok().flatten();
    let prog_map: HashMap<Uuid, _> = progress_list.into_iter().map(|p| (p.ebook_id, p)).collect();

    let items: Vec<EbookCatalogItemDto> = books
        .into_iter()
        .map(|b| {
            let p = prog_map.get(&b.id);
            let unlocked = has_tier_access(&live_tier, &live_role, &b.required_tier);
            let cover_url = b
                .cover_cloudinary_public_id
                .as_deref()
                .map(|pid| generate_optimized_image_url(cloud_creds.as_ref(), pid, 900))
                .unwrap_or(b.cover_image_url);
            EbookCatalogItemDto {
                id: b.id,
                book_code: b.book_code,
                title: b.title,
                original_title: b.original_title,
                author: b.author,
                synopsis: b.synopsis,
                category: b.category,
                required_tier: b.required_tier,
                is_unlocked: unlocked,
                cover_image_url: cover_url,
                total_pages: b.total_pages,
                total_chapters_count: b.total_chapters_count,
                total_figures_count: b.total_figures_count,
                supports_bilingual_toggle: b.supports_bilingual_toggle,
                object_storage_provider: b.object_storage_provider,
                file_size_bytes: b.file_size_bytes,
                pdf_file_size_bytes: b.pdf_file_size_bytes,
                epub_file_size_bytes: b.epub_file_size_bytes,
                sha256_ciphertext_hash: b.sha256_ciphertext_hash,
                toc_chapters: b.toc_chapters_json,
                current_page: p.map(|r| r.current_page).unwrap_or(1),
                current_chapter_number: p.map(|r| r.current_chapter_number).unwrap_or(1),
                completion_pct: p.map(|r| r.completion_pct).unwrap_or(0),
                is_completed: p.is_some_and(|r| r.is_completed),
                bookmarks: p.map(|r| r.bookmarks_json.clone()).unwrap_or_else(|| serde_json::json!([])),
                highlights_notes: p.map(|r| r.highlights_notes_json.clone()).unwrap_or_else(|| serde_json::json!([])),
            }
        })
        .collect();

    Ok(ApiResponse::ok(
        "Pustaka E-Book Barista terproteksi berhasil dimuat.",
        items,
    ))
}

pub async fn request_ebook_license(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(ebook_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<EbookLicenseRequest>,
) -> AppResult<impl IntoResponse> {
    let ebook = ebook_repo::fetch_single_ebook(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;

    if !user_has_tier_access(&state.pool, &claims, &ebook.required_tier).await {
        return Err(AppError::from(BaristaToolkitError::TierUpgradeRequired(
            ebook.required_tier,
        )));
    }

    let cek = unwrap_master_cek(&ebook.wrapped_master_cek, &ebook.book_code, &resolved_drm_secret())
        .map_err(AppError::from)?;
    let client_ikm: Vec<u8> = match req.device_wrapping_secret_b64.as_deref() {
        Some(b64) if !b64.trim().is_empty() => B64
            .decode(b64.trim())
            .map_err(|_| AppError::from(BaristaToolkitError::InvalidBookLicense("Secret perangkat tidak valid.".into())))?,
        _ => state.config.jwt_secret_str().as_bytes().to_vec(),
    };
    let (wrapped_cek_b64, wrap_nonce_b64, aad_context) =
        wrap_cek_for_device(&cek, &req.device_installation_id, claims.sub, ebook.id, &client_ikm)
            .map_err(AppError::from)?;

    let creds = state.config.cloudinary_config().ok().flatten();
    let is_cloud = ebook.object_storage_provider.eq_ignore_ascii_case("CLOUDINARY");
    let use_creds = if is_cloud { creds.as_ref() } else { None };
    let signed_clbk = resolve_protected_book_asset_url(
        use_creds, ebook.clbook_ast_cloudinary_public_id.as_deref(),
        format!("/api/v1/toolkit/ebooks/{}/asset?format=clbook", ebook.id), 3600,
    );
    let signed_pdf = resolve_protected_book_asset_url(
        use_creds, ebook.pdf_cloudinary_public_id.as_deref(),
        format!("/api/v1/toolkit/ebooks/{}/asset?format=pdf", ebook.id), 3600,
    );
    let signed_epub = resolve_protected_book_asset_url(
        use_creds, ebook.epub_cloudinary_public_id.as_deref(),
        format!("/api/v1/toolkit/ebooks/{}/asset?format=epub", ebook.id), 3600,
    );

    Ok(ApiResponse::ok(
        "Lisensi dekripsi AES-256-GCM E-Book berhasil diterbitkan.",
        EbookLicenseResponse {
            ebook_id: ebook.id,
            book_code: ebook.book_code,
            wrapped_cek_b64,
            wrap_nonce_b64,
            aad_context,
            signed_asset_url: signed_clbk,
            signed_pdf_url: signed_pdf,
            signed_epub_url: signed_epub,
            sha256_ciphertext_hash: ebook.sha256_ciphertext_hash,
            pdf_sha256_hash: ebook.pdf_sha256_hash,
            epub_sha256_hash: ebook.epub_sha256_hash,
            expires_at: Utc::now() + Duration::hours(1),
        },
    ))
}

pub async fn sync_ebook_progress(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(ebook_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<SyncEbookProgressRequest>,
) -> AppResult<impl IntoResponse> {
    let ebook = ebook_repo::fetch_single_ebook(&state.pool, ebook_id)
        .await
        .map_err(AppError::from)?;
    let highest = req.highest_page_reached.unwrap_or(req.current_page).max(req.current_page);
    let read_pages = req.read_pages.unwrap_or_default();

    let (completion_pct, is_completed) = ebook_repo::upsert_ebook_progress(
        &state.pool, claims.sub, ebook_id, ebook.total_pages,
        req.current_page, req.current_chapter_number.unwrap_or(1),
        highest, &read_pages, req.bookmarks_json.as_ref(),
        req.highlights_notes_json.as_ref(), req.preferred_reader_mode.as_deref(),
        req.epub_cfi_locator.as_deref(),
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Progres baca & catatan E-Book tersimpan.",
        serde_json::json!({
            "ebook_id": ebook_id,
            "current_page": req.current_page,
            "completion_pct": completion_pct,
            "is_completed": is_completed,
        }),
    ))
}
