use sqlx::PgPool;
use uuid::Uuid;

use crate::{core::error::AppResult, modules::marketplace::models::MarketplaceOrderRow};

pub async fn insert_order(
    pool: &PgPool,
    order_code: &str,
    user_id: Option<Uuid>,
    username: &str,
    recipient_name: &str,
    phone_number: &str,
    shipping_address: &str,
    courier_name: &str,
    payment_method: &str,
    voucher_code: Option<&str>,
    subtotal: i32,
    shipping_fee: i32,
    discount: i32,
    total: i32,
    items_json: serde_json::Value,
) -> AppResult<MarketplaceOrderRow> {
    let row = sqlx::query_as::<_, MarketplaceOrderRow>(
        r#"INSERT INTO marketplace_orders (
            order_code, user_id, username, recipient_name, phone_number, shipping_address,
            courier_name, payment_method, voucher_code, subtotal, shipping_fee, discount, total, items_json
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING *"#,
    )
    .bind(order_code)
    .bind(user_id)
    .bind(username)
    .bind(recipient_name)
    .bind(phone_number)
    .bind(shipping_address)
    .bind(courier_name)
    .bind(payment_method)
    .bind(voucher_code)
    .bind(subtotal)
    .bind(shipping_fee)
    .bind(discount)
    .bind(total)
    .bind(items_json)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_orders(pool: &PgPool, username: &str) -> AppResult<Vec<MarketplaceOrderRow>> {
    let rows = sqlx::query_as::<_, MarketplaceOrderRow>(
        r#"SELECT * FROM marketplace_orders
           WHERE ($1::text = '' OR username = $1)
           ORDER BY created_at DESC
           LIMIT 50"#,
    )
    .bind(username)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
