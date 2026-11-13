use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::recommendation::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/feed/for-you", get(handlers::get_for_you_feed))
        .route("/api/v1/feed/trending", get(handlers::get_trending_feed))
        .route("/api/v1/feed/seen", post(handlers::mark_feed_seen))
        .route("/api/v1/clubs/recommended", get(handlers::get_recommended_clubs))
}
