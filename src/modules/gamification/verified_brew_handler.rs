use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::{ApiResponse, AppResult},
    core::extractors::{AuthUser, ValidatedJson},
    modules::gamification::{
        services::{verified_brew_query_service, verified_brew_service},
        verified_brew_dto::{CompleteVerifiedBrewDto, StartVerifiedBrewDto},
    },
};

pub async fn start_verified_brew_handler(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<StartVerifiedBrewDto>,
) -> AppResult<impl IntoResponse> {
    let data = verified_brew_service::start_session(&pool, claims.sub, req).await?;
    Ok(ApiResponse::created("Sesi seduhan terverifikasi dimulai.", data))
}

pub async fn complete_verified_brew_handler(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
    Json(req): Json<CompleteVerifiedBrewDto>,
) -> AppResult<impl IntoResponse> {
    let data = verified_brew_service::complete_session(&pool, claims.sub, session_id, req).await?;
    Ok(ApiResponse::ok("Seduhan terverifikasi selesai! EXP diberikan.", data))
}

pub async fn get_gamification_me_summary_handler(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let data = verified_brew_query_service::get_me_summary(&pool, claims.sub).await?;
    Ok(ApiResponse::ok("Ringkasan gamifikasi berhasil dimuat.", data))
}
