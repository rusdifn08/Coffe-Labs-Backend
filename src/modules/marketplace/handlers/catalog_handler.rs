use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{error::{ApiResponse, AppResult}, AppState},
    modules::marketplace::{
        dto::MarketplaceProductQuery,
        services::catalog_query_service,
    },
};

pub async fn get_products(
    Query(query): Query<MarketplaceProductQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let products = catalog_query_service::list_verified_products(&state, &query).await?;
    Ok(ApiResponse::ok(
        "Produk Marketplace Specialty Coffee berhasil dimuat.",
        products,
    ))
}

pub async fn get_flash_sale(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    let deals = catalog_query_service::list_flash_sale_deals(&state).await?;
    Ok(ApiResponse::ok("Promo Fresh Roast Flash Sale berhasil dimuat.", deals))
}

pub async fn get_product_detail(
    Path(product_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let detail = catalog_query_service::get_verified_product_detail(&state, product_id).await?;
    Ok(ApiResponse::ok("Detail produk kopi berhasil dimuat.", detail))
}
