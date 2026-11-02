use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        security::JwtKeyRing,
    },
    modules::auth::{
        dto::{ChangePasswordRequest, LoginRequest, RefreshTokenRequest, RegisterRequest},
        models::UserSummary,
        repository, service,
    },
};

pub async fn register(
    State(pool): State<PgPool>,
    State(keyring): State<JwtKeyRing>,
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    let (summary, token, refresh_token) = service::register_user(&pool, &keyring, payload).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Pendaftaran berhasil! Akun Anda aktif dan langsung dapat digunakan login dalam mode Trial.",
            "token": token,
            "refresh_token": refresh_token,
            "user": summary,
            "data": summary
        })),
    ))
}

pub async fn login(
    State(pool): State<PgPool>,
    State(keyring): State<JwtKeyRing>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let resp = service::login_user(&pool, &keyring, payload).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn refresh_token(
    State(pool): State<PgPool>,
    State(keyring): State<JwtKeyRing>,
    ValidatedJson(payload): ValidatedJson<RefreshTokenRequest>,
) -> AppResult<impl IntoResponse> {
    let resp = service::rotate_refresh_token(&pool, &keyring, payload).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn get_me(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let user = repository::find_by_id(&pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User tidak ditemukan.".to_string()))?;
    let summary: UserSummary = user.into();
    Ok(ApiResponse::ok("Data profil berhasil diambil.", summary))
}

pub async fn change_password(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(payload): ValidatedJson<ChangePasswordRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    service::change_user_password(&pool, user_id, payload).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Password berhasil diubah!"
    })))
}

pub async fn list_sessions(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let sessions = service::get_active_sessions(&pool, user_id).await?;
    Ok(ApiResponse::ok("Daftar sesi perangkat aktif berhasil dimuat.", sessions))
}

pub async fn revoke_session_family(
    State(pool): State<PgPool>,
    AuthUser(_claims): AuthUser,
    Path(family_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    repository::revoke_entire_family(&pool, family_id).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Seluruh sesi pada keluarga token tersebut berhasil dihentikan."
    })))
}
