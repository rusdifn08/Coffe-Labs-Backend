use chrono::{Datelike, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, jwt::JwtClaims, AppState},
    modules::marketplace::{
        dto::{
            AddCartItemRequest, CartSummaryDto, PlaceOrderRequest, QrisPaymentSessionDto,
            UpdateCartItemRequest, ValidateVoucherRequest, VoucherValidationResponse,
        },
        error::MarketplaceError,
        models::MarketplaceOrderRow,
        repositories::{cart_repo, order_repo, product_repo},
        services::qris_service,
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
            discount_amount: ((f64::from(subtotal)) * 0.15).round() as i32,
            free_shipping: false,
        }),
        "BARISTA20" => Ok(VoucherValidationResponse {
            valid: true,
            voucher_code: clean,
            label: "BARISTA20 (20% OFF)".to_string(),
            discount_amount: ((f64::from(subtotal)) * 0.20).round() as i32,
            free_shipping: false,
        }),
        "FREESHIP" => Ok(VoucherValidationResponse {
            valid: true,
            voucher_code: clean,
            label: "FREESHIP (Free Courier Delivery)".to_string(),
            discount_amount: 18_000,
            free_shipping: true,
        }),
        other => Err(MarketplaceError::InvalidVoucher(other.to_string()).into()),
    }
}

pub async fn get_cart_summary(pool: &PgPool, user_id: Uuid) -> AppResult<CartSummaryDto> {
    let items = cart_repo::list_user_cart(pool, user_id).await?;
    let subtotal_idr: i64 = items
        .iter()
        .filter(|i| i.selected_for_checkout)
        .map(|i| i.unit_price_idr * i64::from(i.quantity))
        .sum();
    let total_items_count: i32 = items.iter().map(|i| i.quantity).sum();
    Ok(CartSummaryDto {
        items,
        subtotal_idr,
        total_items_count,
    })
}

pub async fn add_item_to_cart(
    pool: &PgPool,
    user_id: Uuid,
    req: &AddCartItemRequest,
) -> AppResult<CartSummaryDto> {
    product_repo::find_verified_product_by_id(pool, req.product_id)
        .await?
        .ok_or(MarketplaceError::ProductNotFound)?;
    cart_repo::upsert_cart_item(pool, user_id, req.product_id, req.variant_id, req.quantity).await?;
    get_cart_summary(pool, user_id).await
}

pub async fn update_cart_item(
    pool: &PgPool,
    user_id: Uuid,
    cart_item_id: Uuid,
    req: &UpdateCartItemRequest,
) -> AppResult<CartSummaryDto> {
    cart_repo::update_cart_item(pool, user_id, cart_item_id, req.quantity, req.selected_for_checkout)
        .await?;
    get_cart_summary(pool, user_id).await
}

pub async fn remove_cart_item(
    pool: &PgPool,
    user_id: Uuid,
    cart_item_id: Uuid,
) -> AppResult<CartSummaryDto> {
    cart_repo::delete_cart_item(pool, user_id, cart_item_id).await?;
    get_cart_summary(pool, user_id).await
}

pub async fn create_qris_order(
    state: &AppState,
    auth: Option<JwtClaims>,
    req: PlaceOrderRequest,
) -> AppResult<QrisPaymentSessionDto> {
    qris_service::enforce_qris_only_payment(&req.payment_method)?;

    let user_id = auth.as_ref().map(|c| c.sub);
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
    let year = Utc::now().year();
    let short_hex = &Uuid::new_v4().simple().to_string()[..6].to_uppercase();
    let order_code = format!("CL-{year}-{short_hex}");
    let qris_ref = format!("QRIS-{year}-{short_hex}");
    let qr_payload = qris_service::build_dynamic_qris_payload(&order_code, total);
    let expires_at = Utc::now() + Duration::minutes(15);

    let order = order_repo::insert_qris_order(
        &state.pool,
        &order_code,
        user_id,
        &username,
        req.recipient_name.trim(),
        req.phone_number.trim(),
        req.shipping_address.trim(),
        req.courier_name.trim(),
        req.voucher_code.as_deref(),
        req.subtotal,
        shipping,
        discount,
        total,
        &qris_ref,
        &qr_payload,
        expires_at,
        req.items,
    )
    .await?;

    Ok(QrisPaymentSessionDto {
        order,
        nmid: qris_service::COFFEE_LABS_QRIS_NMID.to_string(),
        merchant_name: qris_service::COFFEE_LABS_MERCHANT_NAME.to_string(),
        qr_payload,
        expires_at,
    })
}

pub async fn confirm_qris_payment(
    pool: &PgPool,
    order_id_or_code: &str,
) -> AppResult<MarketplaceOrderRow> {
    order_repo::confirm_qris_payment(pool, order_id_or_code).await
}

pub async fn get_user_orders(
    pool: &PgPool,
    auth: Option<JwtClaims>,
) -> AppResult<Vec<MarketplaceOrderRow>> {
    let username = auth.map(|c| c.username).unwrap_or_default();
    order_repo::list_orders(pool, &username).await
}
