use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::modules::marketplace::{
    dto::MarketplaceProductQuery,
    models::{MarketplaceProductRow, ProductVariantRow},
};

pub async fn query_verified_products(
    pool: &PgPool,
    q: &MarketplaceProductQuery,
) -> AppResult<Vec<MarketplaceProductRow>> {
    let search_pat = q
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"));
    let category = q
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("semua") && !s.eq_ignore_ascii_case("all"));
    let roast = q
        .roast_level
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all") && !s.eq_ignore_ascii_case("semua"));
    let process = q
        .process
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all") && !s.eq_ignore_ascii_case("semua"))
        .map(|s| format!("%{s}%"));

    let sort_key = q.sort.as_deref().unwrap_or("relevant");
    let order_clause = match sort_key {
        "latest" => "created_at DESC, id DESC",
        "bestseller" => "sold_count DESC, avg_rating DESC, id DESC",
        "price_asc" => "price_idr ASC, id DESC",
        "price_desc" => "price_idr DESC, id DESC",
        _ => "is_flash_sale DESC, sold_count DESC, avg_rating DESC, id DESC",
    };

    let sql = format!(
        r#"SELECT * FROM marketplace_products
           WHERE is_active = TRUE
             AND image_verification_status = 'VERIFIED_COFFEE_BAG'
             AND ($1::text IS NULL OR title ILIKE $1 OR roastery_name ILIKE $1 OR origin_region ILIKE $1)
             AND ($2::text IS NULL OR category = $2)
             AND ($3::text IS NULL OR roast_level ILIKE $3)
             AND ($4::text IS NULL OR post_harvest_process ILIKE $4)
             AND ($5::bigint IS NULL OR price_idr >= $5)
             AND ($6::bigint IS NULL OR price_idr <= $6)
           ORDER BY {order_clause}
           LIMIT $7 OFFSET $8"#
    );

    let rows = sqlx::query_as::<_, MarketplaceProductRow>(&sql)
        .bind(search_pat)
        .bind(category)
        .bind(roast)
        .bind(process)
        .bind(q.min_price)
        .bind(q.max_price)
        .bind(q.limit.unwrap_or(40).clamp(1, 100))
        .bind(q.offset.unwrap_or(0).max(0))
        .fetch_all(pool)
        .await?;

    Ok(rows)
}

pub async fn list_flash_sale(pool: &PgPool) -> AppResult<Vec<MarketplaceProductRow>> {
    let rows = sqlx::query_as::<_, MarketplaceProductRow>(
        r#"SELECT * FROM marketplace_products
           WHERE is_active = TRUE
             AND image_verification_status = 'VERIFIED_COFFEE_BAG'
             AND is_flash_sale = TRUE
             AND flash_sale_stock_left > 0
           ORDER BY discount_pct DESC, flash_sale_stock_left ASC
           LIMIT 20"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_verified_product_by_id(
    pool: &PgPool,
    product_id: Uuid,
) -> AppResult<Option<MarketplaceProductRow>> {
    let row = sqlx::query_as::<_, MarketplaceProductRow>(
        r#"SELECT * FROM marketplace_products
           WHERE id = $1
             AND is_active = TRUE
             AND image_verification_status = 'VERIFIED_COFFEE_BAG'"#,
    )
    .bind(product_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_variants_for_product(
    pool: &PgPool,
    product_id: Uuid,
) -> AppResult<Vec<ProductVariantRow>> {
    let rows = sqlx::query_as::<_, ProductVariantRow>(
        r#"SELECT * FROM marketplace_product_variants
           WHERE product_id = $1
           ORDER BY weight_grams ASC, grind_option ASC"#,
    )
    .bind(product_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
