use axum::{extract::State, Json};

use crate::core::{
    error::{ApiResponse, AppResult},
    extractors::{RequireTier, TierBusiness},
    AppState,
};
use crate::modules::business_pos::{
    analytics_dto::StoreAnalyticsSummaryResponse,
    analytics_repository::BusinessAnalyticsRepository,
    tenant_context::TenantContext,
};

pub async fn get_analytics_summary(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<StoreAnalyticsSummaryResponse>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let summary = BusinessAnalyticsRepository::get_summary(
        &state.pool,
        tenant.store_id,
        tenant.store_name,
    )
    .await?;

    Ok(ApiResponse::ok("Ringkasan analitik performa kedai berhasil dimuat.", summary))
}
