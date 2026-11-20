use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::modules::marketplace::models::{
    CartItemDetailRow, MarketplaceOrderRow, MarketplaceProductRow, ProductVariantRow,
};

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct MarketplaceProductQuery {
    pub search: Option<String>,
    pub category: Option<String>,
    pub roast_level: Option<String>,
    pub process: Option<String>,
    pub sort: Option<String>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct MarketplaceProductDetailDto {
    #[serde(flatten)]
    pub product: MarketplaceProductRow,
    pub variants: Vec<ProductVariantRow>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ValidateVoucherRequest {
    #[validate(length(min = 2, max = 40))]
    pub voucher_code: String,
    pub subtotal: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VoucherValidationResponse {
    pub valid: bool,
    pub voucher_code: String,
    pub label: String,
    pub discount_amount: i32,
    pub free_shipping: bool,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct PlaceOrderRequest {
    #[validate(length(min = 1, max = 120))]
    pub recipient_name: String,
    #[validate(length(min = 5, max = 40))]
    pub phone_number: String,
    #[validate(length(min = 5, max = 500))]
    pub shipping_address: String,
    pub courier_name: String,
    pub shipping_fee: i32,
    pub payment_method: String,
    pub voucher_code: Option<String>,
    pub subtotal: i32,
    pub items: serde_json::Value,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AddCartItemRequest {
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    #[validate(range(min = 1, max = 99, message = "Kuantitas harus antara 1 dan 99"))]
    pub quantity: i32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateCartItemRequest {
    #[validate(range(min = 1, max = 99))]
    pub quantity: Option<i32>,
    pub selected_for_checkout: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CartSummaryDto {
    pub items: Vec<CartItemDetailRow>,
    pub subtotal_idr: i64,
    pub total_items_count: i32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ConfirmQrisRequest {
    #[allow(dead_code)]
    pub qris_reference: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QrisPaymentSessionDto {
    pub order: MarketplaceOrderRow,
    pub nmid: String,
    pub merchant_name: String,
    pub qr_payload: String,
    pub expires_at: DateTime<Utc>,
}
