use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

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
    pub subtotal: i32,
    pub voucher_code: Option<String>,
    pub items: serde_json::Value,
}
