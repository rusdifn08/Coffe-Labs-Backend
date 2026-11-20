use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::marketplace::models::MarketplaceOrderRow;

#[allow(clippy::too_many_arguments)]
pub async fn insert_qris_order(
    pool: &PgPool,
    order_code: &str,
    user_id: Option<Uuid>,
    username: &str,
    recipient_name: &str,
    phone_number: &str,
    shipping_address: &str,
    courier_name: &str,
    voucher_code: Option<&str>,
    subtotal: i32,
    shipping_fee: i32,
    discount: i32,
    total: i32,
    qris_reference: &str,
    qris_payload: &str,
    qris_expires_at: DateTime<Utc>,
    items_json: serde_json::Value,
) -> AppResult<MarketplaceOrderRow> {
    let row = sqlx::query_as::<_, MarketplaceOrderRow>(
        r#"INSERT INTO marketplace_orders (
            order_code, user_id, username, recipient_name, phone_number, shipping_address,
            courier_name, payment_method, voucher_code, subtotal, shipping_fee, discount, total,
            status, payment_status, qris_reference, qris_payload, qris_expires_at, items_json
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, 'QRIS', $8, $9, $10, $11, $12,
            'Menunggu Pembayaran QRIS', 'PENDING_QRIS', $13, $14, $15, $16
        )
        RETURNING *"#,
    )
    .bind(order_code)
    .bind(user_id)
    .bind(username)
    .bind(recipient_name)
    .bind(phone_number)
    .bind(shipping_address)
    .bind(courier_name)
    .bind(voucher_code)
    .bind(subtotal)
    .bind(shipping_fee)
    .bind(discount)
    .bind(total)
    .bind(qris_reference)
    .bind(qris_payload)
    .bind(qris_expires_at)
    .bind(items_json)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn confirm_qris_payment(
    pool: &PgPool,
    order_id_or_code: &str,
) -> AppResult<MarketplaceOrderRow> {
    let clean = order_id_or_code.trim();
    let row = if let Ok(uuid) = Uuid::parse_str(clean) {
        sqlx::query_as::<_, MarketplaceOrderRow>(
            r#"UPDATE marketplace_orders
               SET payment_status = 'PAID',
                   status = 'Confirmed & Roasting',
                   paid_at = COALESCE(paid_at, NOW())
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(uuid)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query_as::<_, MarketplaceOrderRow>(
            r#"UPDATE marketplace_orders
               SET payment_status = 'PAID',
                   status = 'Confirmed & Roasting',
                   paid_at = COALESCE(paid_at, NOW())
               WHERE order_code = $1
               RETURNING *"#,
        )
        .bind(clean)
        .fetch_optional(pool)
        .await?
    };

    row.ok_or_else(|| AppError::NotFound(format!("Pesanan '{clean}' tidak ditemukan.")))
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
