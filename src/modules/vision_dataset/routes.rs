use axum::{
    routing::{get, post},
    Router,
};
use crate::core::AppState;
use crate::modules::vision_dataset::handlers;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/vision/classes", get(handlers::get_classes))
        .route("/api/v1/vision/classes/sync", post(handlers::sync_class))
}
