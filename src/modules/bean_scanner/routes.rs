use axum::{
    routing::{get, post},
    Router,
};
use crate::core::AppState;
use crate::modules::bean_scanner::handlers;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/beans/scan-resolve", post(handlers::scan_and_resolve_bean))
        .route(
            "/api/v1/beans/transfer-learning/register",
            post(handlers::register_trained_model),
        )
        .route(
            "/api/v1/beans/transfer-learning/models",
            get(handlers::get_trained_models),
        )
}
