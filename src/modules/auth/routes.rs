use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{core::AppState, modules::auth::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/refresh", post(handlers::refresh_token))
        .route("/api/auth/me", get(handlers::get_me))
        .route("/api/auth/change-password", post(handlers::change_password))
        .route("/api/v1/auth/register", post(handlers::register))
        .route("/api/v1/auth/login", post(handlers::login))
        .route("/api/v1/auth/refresh", post(handlers::refresh_token))
        .route("/api/v1/auth/me", get(handlers::get_me))
        .route("/api/v1/auth/change-password", post(handlers::change_password))
        .route("/api/v1/auth/sessions", get(handlers::list_sessions))
        .route("/api/v1/auth/sessions/{family_id}", delete(handlers::revoke_session_family))
}
