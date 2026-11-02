use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::beans::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/beans", get(handlers::get_beans_catalog))
        .route("/api/beans/radar", get(handlers::get_bean_radar))
        .route("/api/beans/dial-in", post(handlers::calculate_ai_dial_in))
        .route("/api/v1/beans", get(handlers::get_beans_catalog))
        .route("/api/v1/beans/radar", get(handlers::get_bean_radar))
        .route("/api/v1/beans/dial-in", post(handlers::calculate_ai_dial_in))
}
