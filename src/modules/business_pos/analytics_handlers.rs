use axum::{extract::State, Json};
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppError, AppResult},
    extractors::{RequireTier, TierBusiness},
    AppState,
};
use crate::modules::business_pos::{
    analytics_dto::StoreAnalyticsSummaryResponse,
    analytics_repository::BusinessAnalyticsRepository,
    tenant_context::TenantContext,
};

fn parse_user_id(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::Unauthorized("ID Pengguna tidak valid.".to_string()))
}

pub async fn get_analytics_summary(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<StoreAnalyticsSummaryResponse>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let tenant = TenantContext::resolve(&state.pool, user_id).await?;

    let summary = BusinessAnalyticsRepository::get_summary(
        &state.pool,
        tenant.store_id,
        tenant.store_name,
    )
    .await?;

    Ok(ApiResponse::ok("Ringkasan analitik performa kedai berhasil dimuat.", summary))
}
