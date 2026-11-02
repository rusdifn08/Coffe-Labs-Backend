use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AdminUser, OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::platform::{
        dto::{
            CreateFeedbackRequest, FeedbackQuery, UpdateAppVersionConfigRequest,
            UpdateFeedbackStatusRequest, VersionCheckQuery,
        },
        models::AppVersionConfig,
        repository, service,
    },
};

pub async fn version_check(
    State(cache): State<Arc<RwLock<AppVersionConfig>>>,
    Query(params): Query<VersionCheckQuery>,
) -> AppResult<impl IntoResponse> {
    let config = cache.read().await;
    let resp = service::evaluate_version_check(&config, &params);
    Ok(Json(resp))
}

pub async fn admin_get_version_config(
    State(cache): State<Arc<RwLock<AppVersionConfig>>>,
    _admin: AdminUser,
) -> AppResult<impl IntoResponse> {
    let config = cache.read().await.clone();
    Ok(ApiResponse::ok(
        "Konfigurasi Version Gatekeeper berhasil dimuat.",
        config,
    ))
}

pub async fn admin_update_version_config(
    State(state): State<AppState>,
    _admin: AdminUser,
    ValidatedJson(payload): ValidatedJson<UpdateAppVersionConfigRequest>,
) -> AppResult<impl IntoResponse> {
    let updated = repository::update_version_config(&state.pool, &payload).await?;
    {
        let mut w = state.version_cache.write().await;
        *w = updated.clone();
    }
    Ok(ApiResponse::ok(
        "Konfigurasi Version Gatekeeper berhasil diperbarui dan diterapkan ke cache instan.",
        updated,
    ))
}

pub async fn submit_feedback(
    State(pool): State<PgPool>,
    OptionalAuthUser(claims_opt): OptionalAuthUser,
    ValidatedJson(payload): ValidatedJson<CreateFeedbackRequest>,
) -> AppResult<impl IntoResponse> {
    let fb = service::create_feedback(&pool, claims_opt, payload).await?;
    Ok(ApiResponse::created(
        "Terima kasih atas masukan Anda! Feedback berhasil dikirim dan akan segera ditinjau oleh Admin Rusdi.",
        fb,
    ))
}

pub async fn admin_get_feedbacks(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Query(params): Query<FeedbackQuery>,
) -> AppResult<impl IntoResponse> {
    let list = service::list_admin_feedbacks(&pool, params).await?;
    Ok(ApiResponse::ok("Berhasil memuat daftar feedback.", list))
}

pub async fn admin_update_feedback_status(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateFeedbackStatusRequest>,
) -> AppResult<impl IntoResponse> {
    let updated = service::update_feedback(&pool, id, payload).await?;
    Ok(ApiResponse::ok(
        format!("Status feedback berhasil diperbarui menjadi '{}'.", updated.status),
        updated,
    ))
}

pub async fn admin_delete_feedback(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service::remove_feedback(&pool, id).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Feedback berhasil dihapus."
    })))
}

pub async fn admin_get_feedback_stats(
    State(pool): State<PgPool>,
    _admin: AdminUser,
) -> AppResult<impl IntoResponse> {
    let stats = service::fetch_stats(&pool).await?;
    Ok(ApiResponse::ok("Statistik feedback berhasil dimuat.", stats))
}
