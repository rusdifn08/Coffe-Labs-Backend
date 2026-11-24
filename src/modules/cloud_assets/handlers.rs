use std::env;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::AuthUser,
        AppState,
    },
    modules::cloud_assets::{
        models::{StorageEndpointsResponse, UpsertDatasetRequest, UpsertMediaBatchRequest},
        repository,
    },
};

const DEFAULT_PRIMARY_CDN: &str =
    "https://cdn.jsdelivr.net/gh/rusdifn08/coffe-labs-mobile@main/cloud_assets";
const DEFAULT_FALLBACK_CDN: &str =
    "https://raw.githubusercontent.com/rusdifn08/coffe-labs-mobile/main/cloud_assets";

#[derive(Debug, Deserialize)]
pub struct MediaListQuery {
    pub prefix: Option<String>,
}

fn ensure_admin(user: &AuthUser) -> AppResult<()> {
    if user.0.role.eq_ignore_ascii_case("admin") {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Hanya admin yang dapat mengubah dataset atau registri object storage.".to_string(),
        ))
    }
}

pub async fn get_storage_config(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let primary = env::var("ASSET_CDN_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| s.starts_with("https://"))
        .unwrap_or_else(|| DEFAULT_PRIMARY_CDN.to_string());

    let mut fallbacks = Vec::new();
    if primary != DEFAULT_PRIMARY_CDN {
        fallbacks.push(DEFAULT_PRIMARY_CDN.to_string());
    }
    fallbacks.push(DEFAULT_FALLBACK_CDN.to_string());

    let provider = if primary.contains("r2.dev") || primary.contains("cloudflare") {
        "CLOUDFLARE_R2"
    } else if primary.contains("cloudinary.com") {
        "CLOUDINARY"
    } else if primary.contains("jsdelivr.net") {
        "JSDELIVR_MULTI_CDN"
    } else {
        "CUSTOM_OBJECT_STORAGE"
    };

    let payload = StorageEndpointsResponse {
        provider: provider.to_string(),
        asset_cdn_base_url: primary,
        asset_cdn_fallback_urls: fallbacks,
        cloudinary_configured: state.cloudinary.is_configured(),
        cloudinary_cloud_name: state.cloudinary.cloud_name().map(str::to_string),
        dataset_api_prefix: "/api/v1/assets/datasets".to_string(),
    };

    Ok(ApiResponse::ok(
        "Konfigurasi Object Storage & Cloud Dataset berhasil dimuat.",
        payload,
    ))
}

pub async fn list_datasets(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let summaries = repository::list_dataset_summaries(&state.pool).await?;
    Ok(ApiResponse::ok(
        "Daftar cloud dataset database berhasil dimuat.",
        summaries,
    ))
}

pub async fn get_dataset(
    State(state): State<AppState>,
    Path(raw_key): Path<String>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let key = repository::validate_dataset_key(&raw_key)?;
    let record = repository::find_dataset_by_key(&state.pool, &key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Dataset '{key}' belum tersedia di database.")))?;

    let etag_value = format!("\"{}\"", record.sha256);
    if let Some(if_none_match) = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
    {
        if if_none_match.trim() == etag_value {
            let mut resp_headers = HeaderMap::new();
            if let Ok(val) = HeaderValue::from_str(&etag_value) {
                resp_headers.insert(header::ETAG, val);
            }
            resp_headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            );
            return Ok((StatusCode::NOT_MODIFIED, resp_headers, Json(json!({}))));
        }
    }

    let mut resp_headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(&etag_value) {
        resp_headers.insert(header::ETAG, val);
    }
    resp_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    Ok((
        StatusCode::OK,
        resp_headers,
        Json(json!({
            "success": true,
            "dataset_key": record.dataset_key,
            "version": record.version,
            "sha256": record.sha256,
            "byte_size": record.byte_size,
            "object_storage_url": record.object_storage_url,
            "updated_at": record.updated_at,
            "data": record.payload_json
        })),
    ))
}

pub async fn upsert_dataset(
    State(state): State<AppState>,
    user: AuthUser,
    Path(raw_key): Path<String>,
    Json(req): Json<UpsertDatasetRequest>,
) -> AppResult<impl IntoResponse> {
    ensure_admin(&user)?;
    let key = repository::validate_dataset_key(&raw_key)?;
    let summary = repository::upsert_dataset(&state.pool, &key, &req).await?;
    Ok(ApiResponse::ok(
        format!("Dataset '{key}' berhasil dikompresi & disimpan ke NeonDB."),
        summary,
    ))
}

pub async fn list_media_assets(
    State(state): State<AppState>,
    Query(q): Query<MediaListQuery>,
) -> AppResult<impl IntoResponse> {
    let rows = repository::list_media_assets(&state.pool, q.prefix.as_deref()).await?;
    Ok(ApiResponse::ok(
        "Registri aset object storage berhasil dimuat.",
        rows,
    ))
}

pub async fn upsert_media_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<UpsertMediaBatchRequest>,
) -> AppResult<impl IntoResponse> {
    ensure_admin(&user)?;
    if req.items.is_empty() || req.items.len() > 500 {
        return Err(AppError::Validation(
            "Batch media harus berisi 1 hingga 500 item.".to_string(),
        ));
    }
    let count = repository::upsert_media_batch(&state.pool, &req.items).await?;
    Ok(ApiResponse::ok(
        format!("{count} aset media berhasil diregistrasi ke database object storage."),
        json!({ "upserted": count }),
    ))
}
