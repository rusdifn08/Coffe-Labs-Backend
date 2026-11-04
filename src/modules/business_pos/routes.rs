use axum::{
    routing::{delete, get, post},
    Router,
};
use crate::core::AppState;
use crate::modules::business_pos::handlers;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/business/config", get(handlers::get_config).put(handlers::update_config))
        .route("/api/v1/business/menu", get(handlers::list_menu).post(handlers::create_menu_item))
        .route("/api/v1/business/menu/{id}", delete(handlers::delete_menu_item))
        .route("/api/v1/business/menu/upload-validate", post(handlers::upload_validate_asset))
        .route("/api/v1/business/pos/orders", post(handlers::create_pos_order))
        .route("/api/v1/business/verify-pin", post(handlers::verify_pin))
}
