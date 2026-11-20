use std::sync::Arc;

use uuid::Uuid;

use crate::core::{
    error::AppResult,
    storage::auto_optimize_url,
    AppState,
};
use crate::modules::marketplace::{
    dto::{MarketplaceProductDetailDto, MarketplaceProductQuery},
    error::MarketplaceError,
    models::MarketplaceProductRow,
    repositories::product_repo,
};

const MKT_FLASH_SALE_CACHE_KEY: &str = "marketplace:flash_sale:verified";

fn optimize_product_images(mut row: MarketplaceProductRow) -> MarketplaceProductRow {
    row.primary_image_url = auto_optimize_url(&row.primary_image_url);
    row
}

pub async fn list_verified_products(
    state: &AppState,
    query: &MarketplaceProductQuery,
) -> AppResult<Vec<MarketplaceProductRow>> {
    let rows = product_repo::query_verified_products(&state.pool, query).await?;
    Ok(rows.into_iter().map(optimize_product_images).collect())
}

pub async fn list_flash_sale_deals(state: &AppState) -> AppResult<Vec<MarketplaceProductRow>> {
    if let Some(cached) = state.caches.directory_l1.get(MKT_FLASH_SALE_CACHE_KEY).await {
        if let Ok(parsed) = serde_json::from_value::<Vec<MarketplaceProductRow>>((*cached).clone()) {
            return Ok(parsed);
        }
    }

    let rows: Vec<MarketplaceProductRow> = product_repo::list_flash_sale(&state.pool)
        .await?
        .into_iter()
        .map(optimize_product_images)
        .collect();

    if let Ok(val) = serde_json::to_value(&rows) {
        state
            .caches
            .directory_l1
            .insert(MKT_FLASH_SALE_CACHE_KEY.to_string(), Arc::new(val))
            .await;
    }

    Ok(rows)
}

pub async fn get_verified_product_detail(
    state: &AppState,
    product_id: Uuid,
) -> AppResult<MarketplaceProductDetailDto> {
    let product = product_repo::find_verified_product_by_id(&state.pool, product_id)
        .await?
        .ok_or(MarketplaceError::ProductNotFound)?;
    let variants = product_repo::list_variants_for_product(&state.pool, product.id).await?;
    Ok(MarketplaceProductDetailDto {
        product: optimize_product_images(product),
        variants,
    })
}
