use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{core::AppState, modules::users::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/admin/users", get(handlers::admin_get_users))
        .route("/api/admin/users/stats", get(handlers::admin_get_user_stats))
        .route("/api/admin/users/{id}/accept", post(handlers::admin_accept_user))
        .route("/api/admin/users/{id}/decline", post(handlers::admin_decline_user))
        .route("/api/admin/users/{id}", delete(handlers::admin_delete_user))
        .route(
            "/api/v1/users/profile",
            get(handlers::get_my_profile).put(handlers::update_my_profile),
        )
}
