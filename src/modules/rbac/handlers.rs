use axum::{extract::State, response::IntoResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
    },
    modules::rbac::{
        dto::UpgradeTierRequest,
        guard::{RequireTier, TierBusiness, TierPro},
        service,
    },
};

pub async fn get_plans() -> AppResult<impl IntoResponse> {
    let plans = service::get_all_plans();
    Ok(ApiResponse::ok("Daftar paket langganan Coffee Labs berhasil dimuat.", plans))
}

pub async fn get_my_subscription(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let status = service::get_subscription_status(&pool, user_id).await?;
    Ok(ApiResponse::ok("Status langganan & kuota berhasil diambil.", status))
}

pub async fn upgrade_tier(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(payload): ValidatedJson<UpgradeTierRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let updated = service::upgrade_user_tier(&pool, user_id, payload).await?;
    Ok(ApiResponse::ok(
        format!("Selamat! Akun Anda berhasil di-upgrade ke tier {:?}.", updated.tier),
        updated,
    ))
}

pub async fn record_brew_quota(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))?;
    let updated = service::record_brew_and_enforce_quota(&pool, user_id).await?;
    Ok(ApiResponse::ok("Kuota seduh berhasil dicatat.", updated))
}

pub async fn verify_pro_access(
    RequireTier(claims, _): RequireTier<TierPro>,
) -> AppResult<impl IntoResponse> {
    Ok(ApiResponse::ok(
        "Akses Coffee Labs Pro terverifikasi.",
        serde_json::json!({ "username": claims.username, "tier": claims.tier }),
    ))
}

pub async fn verify_business_access(
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<impl IntoResponse> {
    Ok(ApiResponse::ok(
        "Akses Coffee Labs Business Cafe & Roaster terverifikasi.",
        serde_json::json!({ "username": claims.username, "tier": claims.tier }),
    ))
}
