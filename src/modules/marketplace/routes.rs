use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::marketplace::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/marketplace/products", get(handlers::get_products))
        .route(
            "/api/v1/marketplace/vouchers/validate",
            post(handlers::validate_voucher),
        )
        .route(
            "/api/v1/marketplace/orders",
            get(handlers::get_orders).post(handlers::place_order),
        )
}
