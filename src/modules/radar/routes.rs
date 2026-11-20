use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::radar::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/radar/nearby", get(handlers::get_nearby_places))
        .route("/api/v1/radar/viewport", get(handlers::get_viewport_places))
        .route("/api/v1/radar/route", get(handlers::get_route_polyline))
        .route("/api/v1/radar/stats", get(handlers::get_radar_stats))
        .route("/api/v1/radar/places/bulk-import", post(handlers::bulk_import_places))
}
