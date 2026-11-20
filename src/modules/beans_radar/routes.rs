use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::beans_radar::handlers::{place_detail_handler, radar_query_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/beans-radar/nearby",
            get(radar_query_handler::get_nearby_places),
        )
        .route(
            "/api/v1/beans-radar/viewport",
            get(radar_query_handler::get_viewport_places),
        )
        .route(
            "/api/v1/beans-radar/search",
            get(radar_query_handler::search_coffee_places),
        )
        .route(
            "/api/v1/beans-radar/stats",
            get(radar_query_handler::get_radar_stats),
        )
        .route(
            "/api/v1/beans-radar/favorites",
            get(place_detail_handler::list_my_favorites),
        )
        .route(
            "/api/v1/beans-radar/places/{id}",
            get(place_detail_handler::get_place_detail),
        )
        .route(
            "/api/v1/beans-radar/places/{id}/favorite",
            post(place_detail_handler::toggle_place_favorite),
        )
}
