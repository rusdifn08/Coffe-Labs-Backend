use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{core::AppState, modules::platform::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/app/version-check", get(handlers::version_check))
        .route(
            "/api/admin/app-version",
            get(handlers::admin_get_version_config).post(handlers::admin_update_version_config),
        )
        .route("/api/feedback", post(handlers::submit_feedback))
        .route("/api/admin/feedbacks", get(handlers::admin_get_feedbacks))
        .route("/api/admin/feedbacks/stats", get(handlers::admin_get_feedback_stats))
        .route(
            "/api/admin/feedbacks/{id}/status",
            post(handlers::admin_update_feedback_status),
        )
        .route("/api/admin/feedbacks/{id}", delete(handlers::admin_delete_feedback))
}
