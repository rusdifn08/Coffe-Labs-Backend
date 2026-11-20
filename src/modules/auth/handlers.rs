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
        jwt::JwtManager,
    },
    modules::auth::{
        dto::{ChangePasswordRequest, LoginRequest, RefreshTokenRequest, RegisterRequest},
        models::UserSummary,
        repository, service,
    },
};

pub async fn register(
    State(pool): State<PgPool>,
    State(jwt): State<JwtManager>,
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> AppResult<impl IntoResponse> {
    let (summary, token, refresh_token) = service::register_user(&pool, &jwt, payload).await?;
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
    State(jwt): State<JwtManager>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let resp = service::login_user(&pool, &jwt, payload).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn refresh_token(
    State(pool): State<PgPool>,
    State(jwt): State<JwtManager>,
    ValidatedJson(payload): ValidatedJson<RefreshTokenRequest>,
) -> AppResult<impl IntoResponse> {
    let resp = service::rotate_refresh_token(&pool, &jwt, payload).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn get_me(
    State(pool): State<PgPool>,
    State(jwt): State<JwtManager>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let user = repository::find_by_id(&pool, claims.sub)
        .await?
        .ok_or_else(|| AppError::NotFound("User tidak ditemukan.".to_string()))?;

    let needs_fresh_token = !user.tier.eq_ignore_ascii_case(&claims.tier)
        || !user.role.eq_ignore_ascii_case(&claims.role)
        || !user.status.eq_ignore_ascii_case(&claims.status);
    let fresh_access_token = if needs_fresh_token {
        jwt.generate_user_token(user.id, &user.username, &user.role, &user.tier, &user.status)
            .ok()
    } else {
        None
    };

    let summary: UserSummary = user.into();
    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Data profil berhasil diambil.",
            "data": summary,
            "fresh_access_token": fresh_access_token
        })),
    ))
}

pub async fn change_password(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(payload): ValidatedJson<ChangePasswordRequest>,
) -> AppResult<impl IntoResponse> {
    service::change_user_password(&pool, claims.sub, payload).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Password berhasil diubah!"
    })))
}

pub async fn list_sessions(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let sessions = service::get_active_sessions(&pool, claims.sub).await?;
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
