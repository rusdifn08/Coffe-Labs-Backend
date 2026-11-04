use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateBusinessConfigDto {
    #[validate(length(min = 2, max = 120, message = "Nama store minimal 2 karakter, maksimal 120."))]
    pub store_name: Option<String>,
    #[validate(range(min = 1, max = 8, message = "Kolom AOD harus antara 1 dan 8."))]
    pub aod_grid_columns: Option<i32>,
    #[validate(range(min = 1, max = 6, message = "Baris AOD harus antara 1 dan 6."))]
    pub aod_grid_rows: Option<i32>,
    pub aod_card_border_color: Option<String>,
    pub aod_accent_color: Option<String>,
    #[validate(range(min = 0.05, max = 1.0, message = "Kecerahan idle antara 0.05 - 1.0."))]
    pub aod_idle_brightness: Option<f32>,
    #[validate(range(min = 0.10, max = 1.0, message = "Kecerahan aktif antara 0.10 - 1.0."))]
    pub aod_active_brightness: Option<f32>,
    pub pixel_shift_enabled: Option<bool>,
    #[validate(length(min = 4, max = 8, message = "PIN kasir baru harus 4-8 digit angka."))]
    pub new_cashier_pin: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateMenuItemDto {
    #[validate(length(min = 2, max = 80, message = "Nama menu minimal 2 karakter, maksimal 80."))]
    pub name: String,
    #[validate(length(max = 240, message = "Deskripsi menu maksimal 240 karakter."))]
    pub description: Option<String>,
    #[validate(length(min = 2, max = 50, message = "Kategori minimal 2 karakter."))]
    pub category: String,
    #[validate(range(min = 0, message = "Harga tidak boleh negatif."))]
    pub price_cents: i64,
    pub currency: Option<String>,
    pub asset_url: String,
    pub asset_format: String,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct UpdateMenuItemDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub price_cents: Option<i64>,
    pub asset_url: Option<String>,
    pub asset_format: Option<String>,
    pub display_order: Option<i32>,
    pub is_available: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PosOrderItemDto {
    pub menu_item_id: Option<Uuid>,
    #[validate(length(min = 1, max = 80))]
    pub item_name: String,
    #[validate(range(min = 1))]
    pub quantity: i32,
    #[validate(range(min = 0))]
    pub unit_price_cents: i64,
    pub modifiers_json: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePosOrderDto {
    pub customer_name: Option<String>,
    #[validate(length(min = 1, message = "Order harus memiliki minimal satu item."))]
    pub items: Vec<PosOrderItemDto>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct VerifyPinDto {
    #[validate(length(min = 4, max = 8, message = "PIN kasir 4-8 digit."))]
    pub pin: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyPinResponse {
    pub verified: bool,
    pub store_name: String,
}
