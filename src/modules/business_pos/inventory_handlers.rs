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
    inventory_dto::{AdjustStockDto, CreateInventoryItemDto, InventoryItemResponse},
    inventory_repository::BusinessInventoryRepository,
    tenant_context::TenantContext,
};

#[derive(Debug, Deserialize)]
pub struct InventoryFilterQuery {
    pub category: Option<String>,
}

pub async fn list_inventory(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Query(query): Query<InventoryFilterQuery>,
) -> AppResult<Json<ApiResponse<Vec<InventoryItemResponse>>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let items = BusinessInventoryRepository::list_by_store(
        &state.pool,
        tenant.store_id,
        query.category.as_deref(),
    )
    .await?;

    let resp: Vec<InventoryItemResponse> = items.into_iter().map(Into::into).collect();
    Ok(ApiResponse::ok("Daftar stok inventori bisnis berhasil diambil.", resp))
}

pub async fn create_inventory_item(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<CreateInventoryItemDto>,
) -> AppResult<Json<ApiResponse<InventoryItemResponse>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let item = BusinessInventoryRepository::create(&state.pool, tenant.store_id, &dto).await?;
    Ok(ApiResponse::ok("Item inventori berhasil didaftarkan.", item.into()))
}

pub async fn adjust_stock(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(item_id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<AdjustStockDto>,
) -> AppResult<Json<ApiResponse<InventoryItemResponse>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let updated = BusinessInventoryRepository::adjust_stock(
        &state.pool,
        tenant.store_id,
        item_id,
        dto.delta_quantity,
    )
    .await?;

    Ok(ApiResponse::ok("Stok inventori berhasil disesuaikan.", updated.into()))
}

pub async fn delete_inventory_item(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(item_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<bool>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let success = BusinessInventoryRepository::delete(&state.pool, tenant.store_id, item_id).await?;
    Ok(ApiResponse::ok("Item inventori berhasil dihapus.", success))
}
