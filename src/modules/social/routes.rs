use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::social::handlers::{feed_handler, interaction_handler, post_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/social/posts",
            get(feed_handler::get_social_posts).post(post_handler::create_social_post),
        )
        .route("/api/v1/social/feed/cursor", get(feed_handler::get_social_posts_cursor))
        .route(
            "/api/v1/social/posts/{id}/like",
            post(interaction_handler::toggle_like_social_post),
        )
        .route(
            "/api/v1/social/posts/{id}/comments",
            get(interaction_handler::get_social_post_comments)
                .post(interaction_handler::add_social_post_comment),
        )
        .route(
            "/api/v1/social/posts/{id}/copy-recipe",
            post(interaction_handler::copy_social_post_recipe),
        )
        .route("/api/v1/social/follow", post(interaction_handler::toggle_follow))
        .route(
            "/api/v1/social/follow/{username}/stats",
            get(interaction_handler::get_follow_stats),
        )
}
