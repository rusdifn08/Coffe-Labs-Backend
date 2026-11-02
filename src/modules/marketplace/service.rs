use chrono::Datelike;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, security::Claims},
    modules::marketplace::{
        dto::{PlaceOrderRequest, ValidateVoucherRequest, VoucherValidationResponse},
        error::MarketplaceError,
        models::MarketplaceOrderRow,
        repository,
    },
};

pub fn evaluate_voucher(req: ValidateVoucherRequest) -> AppResult<VoucherValidationResponse> {
    let clean = req.voucher_code.trim().to_uppercase();
    let subtotal = req.subtotal.max(0);

    match clean.as_str() {
        "COFFEEFIRST" => Ok(VoucherValidationResponse {
            valid: true,
            voucher_code: clean,
            label: "COFFEEFIRST (15% OFF)".to_string(),
            discount_amount: ((subtotal as f64) * 0.15).round() as i32,
            free_shipping: false,
        }),
        "BARISTA20" => Ok(VoucherValidationResponse {
            valid: true,
            voucher_code: clean,
            label: "BARISTA20 (20% OFF)".to_string(),
            discount_amount: ((subtotal as f64) * 0.20).round() as i32,
            free_shipping: false,
        }),
        "FREESHIP" => Ok(VoucherValidationResponse {
            valid: true,
            voucher_code: clean,
            label: "FREESHIP (Free Courier Delivery)".to_string(),
            discount_amount: 18000,
            free_shipping: true,
        }),
        other => Err(MarketplaceError::InvalidVoucher(other.to_string()).into()),
    }
}

pub async fn create_order(
    pool: &PgPool,
    auth: Option<Claims>,
    req: PlaceOrderRequest,
) -> AppResult<MarketplaceOrderRow> {
    let user_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let username = auth
        .map(|c| c.username)
        .unwrap_or_else(|| "guest".to_string());

    let mut discount = 0;
    let mut shipping = req.shipping_fee.max(0);

    if let Some(ref code) = req.voucher_code {
        if let Ok(v) = evaluate_voucher(ValidateVoucherRequest {
            voucher_code: code.clone(),
            subtotal: req.subtotal,
        }) {
            discount = v.discount_amount;
            if v.free_shipping {
                shipping = 0;
            }
        }
    }

    let total = (req.subtotal + shipping - discount).max(0);
    let year = chrono::Utc::now().year();
    let short_hex = &Uuid::new_v4().simple().to_string()[..6].to_uppercase();
    let order_code = format!("CL-{}-{}", year, short_hex);

    repository::insert_order(
        pool,
        &order_code,
        user_id,
        &username,
        req.recipient_name.trim(),
        req.phone_number.trim(),
        req.shipping_address.trim(),
        req.courier_name.trim(),
        req.payment_method.trim(),
        req.voucher_code.as_deref(),
        req.subtotal,
        shipping,
        discount,
        total,
        req.items,
    )
    .await
}

pub async fn get_user_orders(
    pool: &PgPool,
    auth: Option<Claims>,
) -> AppResult<Vec<MarketplaceOrderRow>> {
    let username = auth.map(|c| c.username).unwrap_or_default();
    repository::list_orders(pool, &username).await
}
