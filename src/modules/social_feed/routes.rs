use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use crate::{
    core::AppState,
    modules::social_feed::handlers::{interaction_handler, profile_handler, timeline_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/social/profiles",
            get(profile_handler::explore_profiles),
        )
        .route(
            "/api/v1/social/profiles/me",
            patch(profile_handler::update_my_profile),
        )
        .route(
            "/api/v1/social/profiles/{target}",
            get(profile_handler::get_profile),
        )
        .route(
            "/api/v1/social/profiles/{target}/follow",
            post(profile_handler::toggle_follow),
        )
        .route(
            "/api/v1/social/profiles/{target}/followers",
            get(profile_handler::list_followers),
        )
        .route(
            "/api/v1/social/profiles/{target}/following",
            get(profile_handler::list_following),
        )
        .route(
            "/api/v1/social/profiles/{target}/threads",
            get(timeline_handler::get_profile_tab_threads),
        )
        .route(
            "/api/v1/social/threads/timeline",
            get(timeline_handler::get_global_timeline),
        )
        .route(
            "/api/v1/social/threads",
            post(interaction_handler::create_thread),
        )
        .route(
            "/api/v1/social/threads/{id}",
            get(timeline_handler::get_thread_detail),
        )
        .route(
            "/api/v1/social/threads/{id}/delete",
            delete(interaction_handler::delete_thread),
        )
        .route(
            "/api/v1/social/threads/{id}/like",
            post(interaction_handler::toggle_like),
        )
        .route(
            "/api/v1/social/threads/{id}/repost",
            post(interaction_handler::toggle_repost),
        )
        .route(
            "/api/v1/social/threads/{id}/try-recipe",
            post(interaction_handler::try_recipe),
        )
}
