use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppResult},
    extractors::{RequireTier, TierBusiness, ValidatedJson},
    AppState,
};
use crate::modules::business_pos::{
    table_dto::{CreateTableDto, UpdateTableStatusDto},
    table_models::BusinessDiningTable,
    table_repository::BusinessTableRepository,
    tenant_context::TenantContext,
};

pub async fn list_tables(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Vec<BusinessDiningTable>>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let tables = BusinessTableRepository::list_by_store(&state.pool, tenant.store_id).await?;
    Ok(ApiResponse::ok("Daftar meja kedai berhasil dimuat.", tables))
}

pub async fn create_table(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<CreateTableDto>,
) -> AppResult<Json<ApiResponse<BusinessDiningTable>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let table = BusinessTableRepository::create(&state.pool, tenant.store_id, &dto).await?;
    Ok(ApiResponse::ok("Meja baru berhasil ditambahkan ke tata ruang kedai.", table))
}

pub async fn update_table_status(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(table_id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<UpdateTableStatusDto>,
) -> AppResult<Json<ApiResponse<BusinessDiningTable>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let updated = BusinessTableRepository::update_status(
        &state.pool,
        tenant.store_id,
        table_id,
        &dto,
    )
    .await?;

    Ok(ApiResponse::ok("Status meja berhasil diperbarui.", updated))
}

pub async fn delete_table(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(table_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<bool>>> {
    let tenant = TenantContext::resolve(&state.pool, claims.sub).await?;

    let success = BusinessTableRepository::delete(&state.pool, tenant.store_id, table_id).await?;
    Ok(ApiResponse::ok("Meja berhasil dihapus.", success))
}
