use axum::{routing::post, Router};
use crate::core::AppState;
use crate::modules::bean_scanner::handlers;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/beans/scan-resolve", post(handlers::scan_and_resolve_bean))
}
