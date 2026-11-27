use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::routing::handlers::routing_handler};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/routing/compare-modes",
            post(routing_handler::compare_modes),
        )
        .route(
            "/api/v1/routing/navigate",
            post(routing_handler::navigate_live),
        )
        .route(
            "/api/v1/routing/reverse-geocode",
            get(routing_handler::reverse_geocode),
        )
}
