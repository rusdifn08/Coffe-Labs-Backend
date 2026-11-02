use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{core::AppState, modules::recipes::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/recipes",
            get(handlers::get_synced_recipes).post(handlers::upsert_custom_recipe),
        )
        .route("/api/recipes/sync", get(handlers::get_synced_recipes))
        .route("/api/recipes/{id}", delete(handlers::delete_custom_recipe))
        .route("/api/recipes/{id}/favorite", post(handlers::toggle_favorite_recipe))
}
