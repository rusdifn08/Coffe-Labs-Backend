use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    core::AppState,
    modules::marketplace::handlers::{cart_checkout_handler, catalog_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/marketplace/products", get(catalog_handler::get_products))
        .route("/api/v1/marketplace/flash-sale", get(catalog_handler::get_flash_sale))
        .route("/api/v1/marketplace/products/{id}", get(catalog_handler::get_product_detail))
        .route("/api/v1/marketplace/cart", get(cart_checkout_handler::get_cart))
        .route("/api/v1/marketplace/cart/items", post(cart_checkout_handler::add_cart_item))
        .route(
            "/api/v1/marketplace/cart/items/{id}",
            patch(cart_checkout_handler::update_cart_item)
                .delete(cart_checkout_handler::remove_cart_item),
        )
        .route(
            "/api/v1/marketplace/vouchers/validate",
            post(cart_checkout_handler::validate_voucher),
        )
        .route("/api/v1/marketplace/checkout", post(cart_checkout_handler::place_order))
        .route(
            "/api/v1/marketplace/orders",
            get(cart_checkout_handler::get_orders).post(cart_checkout_handler::place_order),
        )
        .route(
            "/api/v1/marketplace/orders/{id_or_code}/qris/confirm",
            post(cart_checkout_handler::confirm_qris_payment),
        )
}
