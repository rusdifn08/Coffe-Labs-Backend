use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::telemetry::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::serve_dashboard))
        .route("/dashboard", get(handlers::serve_dashboard))
        .route("/health", get(handlers::health))
        .route("/api/health", get(handlers::health))
        .route("/api/v1/telemetry/metrics", get(handlers::get_telemetry_metrics))
        .route("/api/v1/telemetry/reset", post(handlers::reset_telemetry_metrics))
        .route("/api/v1/openapi.json", get(handlers::serve_openapi))
}
