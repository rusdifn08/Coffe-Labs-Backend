use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::modules::marketplace::models::CartItemDetailRow;

pub async fn list_user_cart(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<CartItemDetailRow>> {
    let rows = sqlx::query_as::<_, CartItemDetailRow>(
        r#"SELECT c.id,
                  c.user_id,
                  c.product_id,
                  c.variant_id,
                  p.title AS product_title,
                  p.roastery_name,
                  p.primary_image_url,
                  COALESCE(v.weight_grams, p.base_weight_grams) AS weight_grams,
                  COALESCE(v.grind_option, 'Biji Utuh (Whole Bean)') AS grind_option,
                  COALESCE(v.price_idr, p.price_idr) AS unit_price_idr,
                  c.quantity,
                  c.selected_for_checkout
           FROM marketplace_cart_items c
           JOIN marketplace_products p ON p.id = c.product_id
           LEFT JOIN marketplace_product_variants v ON v.id = c.variant_id
           WHERE c.user_id = $1
             AND p.is_active = TRUE
             AND p.image_verification_status = 'VERIFIED_COFFEE_BAG'
           ORDER BY c.created_at DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_cart_item(
    pool: &PgPool,
    user_id: Uuid,
    product_id: Uuid,
    variant_id: Option<Uuid>,
    quantity: i32,
) -> AppResult<()> {
    let new_id = Uuid::now_v7();
    sqlx::query(
        r#"INSERT INTO marketplace_cart_items (id, user_id, product_id, variant_id, quantity)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (user_id, product_id, variant_id)
           DO UPDATE SET
               quantity = LEAST(99, marketplace_cart_items.quantity + EXCLUDED.quantity),
               updated_at = NOW()"#,
    )
    .bind(new_id)
    .bind(user_id)
    .bind(product_id)
    .bind(variant_id)
    .bind(quantity)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_cart_item(
    pool: &PgPool,
    user_id: Uuid,
    cart_item_id: Uuid,
    quantity: Option<i32>,
    selected: Option<bool>,
) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE marketplace_cart_items
           SET quantity = COALESCE($3, quantity),
               selected_for_checkout = COALESCE($4, selected_for_checkout),
               updated_at = NOW()
           WHERE id = $1 AND user_id = $2"#,
    )
    .bind(cart_item_id)
    .bind(user_id)
    .bind(quantity)
    .bind(selected)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_cart_item(
    pool: &PgPool,
    user_id: Uuid,
    cart_item_id: Uuid,
) -> AppResult<()> {
    sqlx::query("DELETE FROM marketplace_cart_items WHERE id = $1 AND user_id = $2")
        .bind(cart_item_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
