use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        AppState,
    },
    modules::admin_control::{
        dto::{AuditLogDto, ControlSnapshotDto, DynamicPlanPricingDto, UpdatePlanPriceDto},
        middleware::admin_role_guard::RequireAdminUser,
        repositories::pricing_control_repo,
        services::pricing_mutation_service,
    },
};

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
}

pub async fn list_admin_pricing(
    _admin: RequireAdminUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<DynamicPlanPricingDto>>>> {
    let plans = pricing_control_repo::list_all_plans(&state.pool)
        .await?
        .into_iter()
        .map(DynamicPlanPricingDto::from)
        .collect();
    Ok(ApiResponse::ok(
        "Daftar harga paket Pro & Business diperoleh.",
        plans,
    ))
}

pub async fn update_plan_pricing(
    admin: RequireAdminUser,
    State(state): State<AppState>,
    Path(plan_code): Path<String>,
    Json(req): Json<UpdatePlanPriceDto>,
) -> AppResult<Json<ApiResponse<ControlSnapshotDto>>> {
    let snap = pricing_mutation_service::update_plan_pricing(
        &state,
        admin.user_id,
        &admin.admin_label,
        &plan_code,
        req,
    )
    .await?;
    Ok(ApiResponse::ok(
        format!("Harga paket {plan_code} berhasil diperbarui dan disiarkan real-time."),
        snap,
    ))
}

pub async fn list_audit_logs(
    _admin: RequireAdminUser,
    State(state): State<AppState>,
    Query(q): Query<AuditLogQuery>,
) -> AppResult<Json<ApiResponse<Vec<AuditLogDto>>>> {
    let limit = q.limit.unwrap_or(40).clamp(1, 100);
    let logs = pricing_control_repo::list_recent_audit_logs(&state.pool, limit)
        .await?
        .into_iter()
        .map(AuditLogDto::from)
        .collect();
    Ok(ApiResponse::ok(
        "Riwayat audit perubahan Control Panel Admin diperoleh.",
        logs,
    ))
}
