use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct MarketplaceOrderRow {
    pub id: Uuid,
    pub order_code: String,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub recipient_name: String,
    pub phone_number: String,
    pub shipping_address: String,
    pub courier_name: String,
    pub payment_method: String,
    pub voucher_code: Option<String>,
    pub subtotal: i32,
    pub shipping_fee: i32,
    pub discount: i32,
    pub total: i32,
    pub status: String,
    pub items_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
