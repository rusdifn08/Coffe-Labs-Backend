use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MarketplaceProductRow {
    pub id: Uuid,
    pub roastery_id: Option<Uuid>,
    pub roastery_name: String,
    pub is_mall_roastery: bool,
    pub city_origin: String,
    pub sku_slug: String,
    pub title: String,
    pub category: String,
    pub origin_region: String,
    pub altitude_masl: String,
    pub varieties: serde_json::Value,
    pub post_harvest_process: String,
    pub roast_level: String,
    pub tasting_notes: serde_json::Value,
    pub description: String,
    pub base_weight_grams: i32,
    pub price_idr: i64,
    pub original_price_idr: Option<i64>,
    pub discount_pct: i16,
    pub is_flash_sale: bool,
    pub flash_sale_stock_left: i32,
    pub stock_qty: i32,
    pub sold_count: i32,
    pub avg_rating: f32,
    pub reviews_count: i32,
    pub primary_image_url: String,
    pub primary_cloudinary_public_id: Option<String>,
    pub gallery_image_urls: serde_json::Value,
    pub image_phash_hex: Option<String>,
    pub image_ai_bag_confidence: f32,
    pub image_face_detected: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ProductVariantRow {
    pub id: Uuid,
    pub product_id: Uuid,
    pub weight_grams: i32,
    pub grind_option: String,
    pub price_idr: i64,
    pub stock_qty: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CartItemDetailRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub product_title: String,
    pub roastery_name: String,
    pub primary_image_url: String,
    pub weight_grams: i32,
    pub grind_option: String,
    pub unit_price_idr: i64,
    pub quantity: i32,
    pub selected_for_checkout: bool,
}

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
    pub payment_status: String,
    pub qris_reference: Option<String>,
    pub qris_payload: Option<String>,
    pub qris_expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub items_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
