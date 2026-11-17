use std::sync::Arc;

use axum::{
    extract::{FromRef, FromRequestParts, Path, Query, State},
    http::request::Parts,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::AuthUser,
        jwt::{JwtClaims, JwtManager},
        AppState,
    },
    modules::monitoring::{
        dto::AccountListQuery,
        error::MonitoringError,
        repositories::monitoring_query_repo,
    },
};

#[derive(Debug, Clone)]
pub struct MonitoringViewer(#[allow(dead_code)] pub JwtClaims);

impl<S> FromRequestParts<S> for MonitoringViewer
where
    S: Send + Sync,
    JwtManager: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        let is_allowed =
            claims.role.eq_ignore_ascii_case("admin") || claims.tier.eq_ignore_ascii_case("business");
        if !is_allowed {
            return Err(AppError::from(MonitoringError::Forbidden));
        }
        Ok(MonitoringViewer(claims))
    }
}

pub async fn list_monitored_accounts(
    State(state): State<AppState>,
    _viewer: MonitoringViewer,
    Query(query): Query<AccountListQuery>,
) -> AppResult<impl IntoResponse> {
    let cache_key = format!(
        "q={}|t={}|r={}|o={}|p={}|pp={}",
        query.q.as_deref().unwrap_or(""),
        query.tier.as_deref().unwrap_or(""),
        query.risk.as_deref().unwrap_or(""),
        query.online_only.unwrap_or(false),
        query.page.unwrap_or(1),
        query.per_page.unwrap_or(25),
    );

    if let Some(hit) = state.monitoring.l1_caches.accounts_page.get(&cache_key).await {
        return Ok(ApiResponse::ok(
            "Daftar monitoring akun (L1 cache) berhasil dimuat.",
            hit.as_ref().clone(),
        ));
    }

    let live_map = state.monitoring.live_presence.live_snapshot_map().await;
    let page = monitoring_query_repo::query_accounts_page(&state.pool, &query, &live_map)
        .await
        .map_err(AppError::from)?;

    let arc = Arc::new(page);
    state
        .monitoring
        .l1_caches
        .accounts_page
        .insert(cache_key, Arc::clone(&arc))
        .await;

    Ok(ApiResponse::ok(
        "Daftar monitoring akun real-time berhasil dimuat.",
        arc.as_ref().clone(),
    ))
}

pub async fn get_account_dossier(
    State(state): State<AppState>,
    _viewer: MonitoringViewer,
    Path(user_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    if let Some(hit) = state.monitoring.l1_caches.dossiers.get(&user_id).await {
        return Ok(ApiResponse::ok(
            "Dossier 360 akun (L1 cache) berhasil dimuat.",
            hit.as_ref().clone(),
        ));
    }

    let live_map = state.monitoring.live_presence.live_snapshot_map().await;
    let dossier = monitoring_query_repo::query_account_360_dossier(
        &state.pool,
        user_id,
        live_map.get(&user_id),
    )
    .await
    .map_err(AppError::from)?;

    let arc = Arc::new(dossier);
    state
        .monitoring
        .l1_caches
        .dossiers
        .insert(user_id, Arc::clone(&arc))
        .await;

    Ok(ApiResponse::ok(
        "Dossier 360 akun berhasil dimuat.",
        arc.as_ref().clone(),
    ))
}
