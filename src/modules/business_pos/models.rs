#![allow(dead_code)]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessStoreConfig {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub store_name: String,
    #[serde(skip_serializing)]
    pub cashier_pin_hash: String,
    pub aod_grid_columns: i32,
    pub aod_grid_rows: i32,
    pub aod_card_border_color: String,
    pub aod_accent_color: String,
    pub aod_idle_brightness: f32,
    pub aod_active_brightness: f32,
    pub pixel_shift_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessMenuItem {
    pub id: Uuid,
    pub store_id: Uuid,
    pub name: String,
    pub description: String,
    pub category: String,
    pub price_cents: i64,
    pub currency: String,
    pub asset_url: String,
    pub asset_format: String,
    pub has_verified_transparency: bool,
    pub display_order: i32,
    pub is_available: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PosOrder {
    pub id: Uuid,
    pub store_id: Uuid,
    pub order_number: String,
    pub cashier_user_id: Option<Uuid>,
    pub customer_name: Option<String>,
    pub subtotal_cents: i64,
    pub tax_cents: i64,
    pub total_cents: i64,
    pub payment_method: String,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PosOrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub menu_item_id: Option<Uuid>,
    pub item_name: String,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub subtotal_cents: i64,
    pub modifiers_json: Option<serde_json::Value>,
}
