use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::marketplace::{
        dto::{
            AddCartItemRequest, ConfirmQrisRequest, PlaceOrderRequest, UpdateCartItemRequest,
            ValidateVoucherRequest,
        },
        services::cart_checkout_service,
    },
};

pub async fn validate_voucher(
    ValidatedJson(payload): ValidatedJson<ValidateVoucherRequest>,
) -> AppResult<impl IntoResponse> {
    let res = cart_checkout_service::evaluate_voucher(payload)?;
    Ok(ApiResponse::ok("Voucher berhasil divalidasi.", res))
}

pub async fn get_cart(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let cart = cart_checkout_service::get_cart_summary(&state.pool, claims.sub).await?;
    Ok(ApiResponse::ok("Keranjang belanja berhasil dimuat.", cart))
}

pub async fn add_cart_item(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<AddCartItemRequest>,
) -> AppResult<impl IntoResponse> {
    let cart = cart_checkout_service::add_item_to_cart(&state.pool, claims.sub, &payload).await?;
    Ok(ApiResponse::created("Produk ditambahkan ke keranjang.", cart))
}

pub async fn update_cart_item(
    AuthUser(claims): AuthUser,
    Path(item_id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateCartItemRequest>,
) -> AppResult<impl IntoResponse> {
    let cart = cart_checkout_service::update_cart_item(&state.pool, claims.sub, item_id, &payload).await?;
    Ok(ApiResponse::ok("Kuantitas keranjang diperbarui.", cart))
}

pub async fn remove_cart_item(
    AuthUser(claims): AuthUser,
    Path(item_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let cart = cart_checkout_service::remove_cart_item(&state.pool, claims.sub, item_id).await?;
    Ok(ApiResponse::ok("Item dihapus dari keranjang.", cart))
}

pub async fn place_order(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<PlaceOrderRequest>,
) -> AppResult<impl IntoResponse> {
    let session = cart_checkout_service::create_qris_order(&state, auth, payload).await?;
    Ok(ApiResponse::created(
        "Pesanan dibuat. Silakan selesaikan pembayaran via QRIS.",
        session,
    ))
}

pub async fn confirm_qris_payment(
    Path(order_id_or_code): Path<String>,
    State(state): State<AppState>,
    ValidatedJson(_payload): ValidatedJson<ConfirmQrisRequest>,
) -> AppResult<impl IntoResponse> {
    let order = cart_checkout_service::confirm_qris_payment(&state.pool, &order_id_or_code).await?;
    Ok(ApiResponse::ok("Pembayaran QRIS terkonfirmasi lunas.", order))
}

pub async fn get_orders(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let orders = cart_checkout_service::get_user_orders(&state.pool, auth).await?;
    Ok(ApiResponse::ok("Daftar pesanan marketplace berhasil dimuat.", orders))
}
