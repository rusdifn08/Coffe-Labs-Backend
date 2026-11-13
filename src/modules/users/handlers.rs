use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AdminUser, AuthUser, ValidatedJson},
        AppState,
    },
    modules::{
        notifications::builders::build_tier_upgrade_event,
        users::{
            dto::{UpdateProfileRequest, UpdateUserTierRequest},
            service,
        },
    },
};

pub async fn admin_get_users(
    State(pool): State<PgPool>,
    _admin: AdminUser,
) -> AppResult<impl IntoResponse> {
    let list = service::get_all_users(&pool).await?;
    Ok(ApiResponse::ok("Daftar pengguna berhasil diambil.", list))
}

pub async fn admin_get_user_stats(
    State(pool): State<PgPool>,
    _admin: AdminUser,
) -> AppResult<impl IntoResponse> {
    let stats = service::get_user_stats(&pool).await?;
    Ok(ApiResponse::ok("Statistik pengguna berhasil diambil.", stats))
}

pub async fn admin_accept_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let summary = service::accept_user(&pool, user_id).await?;
    Ok(ApiResponse::ok(
        format!("Akun '{}' berhasil disetujui (Approved).", summary.username),
        summary,
    ))
}

pub async fn admin_decline_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let summary = service::decline_user(&pool, user_id).await?;
    Ok(ApiResponse::ok(
        format!("Pendaftaran akun '{}' berhasil ditolak (Declined).", summary.username),
        summary,
    ))
}

pub async fn admin_delete_user(
    State(pool): State<PgPool>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    service::delete_user(&pool, user_id).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Pengguna berhasil dihapus."
    })))
}

pub async fn admin_update_user_tier(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateUserTierRequest>,
) -> AppResult<impl IntoResponse> {
    let summary = service::update_user_tier(&state.pool, user_id, &req.tier).await?;
    state
        .notifications
        .publish(build_tier_upgrade_event(user_id, &summary.tier));
    Ok(ApiResponse::ok(
        format!("Tier akun '{}' berhasil diperbarui menjadi '{}'.", summary.username, summary.tier),
        summary,
    ))
}

pub async fn get_my_profile(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let profile = service::get_barista_profile(&pool, user_id).await?;
    Ok(ApiResponse::ok("Profil barista berhasil dimuat.", profile))
}

pub async fn update_my_profile(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(payload): ValidatedJson<UpdateProfileRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let profile = service::update_barista_profile(&pool, user_id, payload).await?;
    Ok(ApiResponse::ok("Profil barista berhasil diperbarui.", profile))
}
