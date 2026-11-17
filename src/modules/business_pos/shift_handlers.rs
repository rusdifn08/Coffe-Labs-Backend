use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppResult},
    extractors::{RequireTier, TierBusiness, ValidatedJson},
    AppState,
};
use crate::modules::business_pos::{
    shift_dto::{CloseShiftDto, OpenShiftDto},
    shift_models::BusinessCashierShift,
    shift_repository::BusinessShiftRepository,
    tenant_context::TenantContext,
};

#[derive(Debug, Deserialize)]
pub struct ShiftHistoryQuery {
    pub limit: Option<i64>,
}

pub async fn get_current_shift(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Option<BusinessCashierShift>>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let shift = BusinessShiftRepository::get_active_shift(&state.pool, tenant.store_id).await?;
    Ok(ApiResponse::ok("Status shift kasir aktif berhasil diambil.", shift))
}

pub async fn open_shift(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<OpenShiftDto>,
) -> AppResult<Json<ApiResponse<BusinessCashierShift>>> {
    let user_id = claims.sub;
    let tenant = TenantContext::resolve(&state.pool, user_id).await?;
    let cashier_name = if claims.username.is_empty() {
        user_id.to_string()
    } else {
        claims.username
    };

    let shift = BusinessShiftRepository::open_shift(
        &state.pool,
        tenant.store_id,
        user_id,
        &cashier_name,
        &dto,
    )
    .await?;

    Ok(ApiResponse::ok("Shift kasir baru berhasil dibuka.", shift))
}

pub async fn close_shift(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(shift_id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<CloseShiftDto>,
) -> AppResult<Json<ApiResponse<BusinessCashierShift>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let closed = BusinessShiftRepository::close_shift(
        &state.pool,
        tenant.store_id,
        shift_id,
        &dto,
    )
    .await?;

    Ok(ApiResponse::ok("Shift kasir berhasil ditutup dan direkonsiliasi.", closed))
}

pub async fn list_shift_history(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Query(query): Query<ShiftHistoryQuery>,
) -> AppResult<Json<ApiResponse<Vec<BusinessCashierShift>>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let list = BusinessShiftRepository::list_history(
        &state.pool,
        tenant.store_id,
        query.limit.unwrap_or(20),
    )
    .await?;

    Ok(ApiResponse::ok("Riwayat shift kasir berhasil dimuat.", list))
}
