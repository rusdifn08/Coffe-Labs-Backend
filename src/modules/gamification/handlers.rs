use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
    },
    modules::{
        gamification::{
            dto::{
                ClaimDomainActionRequest, CompleteBrewSessionRequest, LeaderboardQuery,
                StartBrewSessionRequest,
            },
            repository,
            services::{anti_cheat_service, leaderboard_service},
        },
        rbac::models::AccountTier,
    },
};

fn parse_uid(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::BadRequest("ID user tidak valid.".to_string()))
}

pub async fn get_my_profile(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let tier = AccountTier::from_str_lossy(&claims.tier);
    let data = leaderboard_service::get_full_profile(
        &pool,
        uid,
        &claims.username,
        tier,
        "Asia/Jakarta",
    )
    .await?;
    Ok(ApiResponse::ok("Profil gamifikasi Barista berhasil dimuat.", data))
}

pub async fn start_brew_session_handler(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<StartBrewSessionRequest>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let data = anti_cheat_service::start_brew_session(&pool, uid, req).await?;
    Ok(ApiResponse::created(
        "Sesi verifikasi seduhan dimulai. Timer anti-cheat aktif.",
        data,
    ))
}

pub async fn complete_brew_session_handler(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<CompleteBrewSessionRequest>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let tier = AccountTier::from_str_lossy(&claims.tier);
    let data = anti_cheat_service::complete_brew_session(&pool, uid, tier, req).await?;
    Ok(ApiResponse::ok(
        "Verifikasi seduhan sukses! XP dan progres lencana diperbarui.",
        data,
    ))
}

pub async fn claim_domain_action_handler(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<ClaimDomainActionRequest>,
) -> AppResult<impl IntoResponse> {
    let uid = parse_uid(&claims.sub)?;
    let tier = AccountTier::from_str_lossy(&claims.tier);
    let data = anti_cheat_service::claim_domain_action(&pool, uid, tier, req).await?;
    Ok(ApiResponse::ok("Reward aktivitas berhasil diklaim!", data))
}

pub async fn get_leaderboard_handler(
    State(pool): State<PgPool>,
    Query(query): Query<LeaderboardQuery>,
) -> AppResult<impl IntoResponse> {
    let scope = query.scope.as_deref().unwrap_or("global");
    let limit = query.limit.unwrap_or(100);
    let rows = leaderboard_service::get_cached_leaderboard(&pool, scope, limit).await?;
    Ok(ApiResponse::ok("Leaderboard Barista berhasil dimuat.", rows))
}

pub async fn get_badges_catalog_handler(
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let rows = repository::list_catalog(&pool).await?;
    Ok(ApiResponse::ok("Katalog 20 Lencana Specialty Coffee berhasil dimuat.", rows))
}
