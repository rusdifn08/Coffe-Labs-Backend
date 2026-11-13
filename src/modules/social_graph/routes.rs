use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::social_graph::handlers::{action_handler, query_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/social/graph/relationship/{target}",
            get(query_handler::get_relationship_snapshot),
        )
        .route(
            "/api/v1/social/graph/relationships/batch",
            post(query_handler::post_batch_relationships),
        )
        .route(
            "/api/v1/social/graph/users/{target}/follow",
            post(action_handler::post_follow).delete(action_handler::delete_follow),
        )
        .route(
            "/api/v1/social/graph/users/{target}/followers",
            get(query_handler::get_followers_list),
        )
        .route(
            "/api/v1/social/graph/users/{target}/following",
            get(query_handler::get_following_list),
        )
        .route(
            "/api/v1/social/graph/users/{target}/mutuals",
            get(query_handler::get_mutual_connections_list),
        )
        .route(
            "/api/v1/social/graph/requests/pending",
            get(query_handler::get_pending_requests),
        )
        .route(
            "/api/v1/social/graph/requests/{requester_id}/accept",
            post(action_handler::post_accept_request),
        )
        .route(
            "/api/v1/social/graph/requests/{requester_id}/reject",
            post(action_handler::post_reject_request),
        )
        .route(
            "/api/v1/social/graph/followers/{follower_id}",
            delete(action_handler::delete_follower),
        )
        .route(
            "/api/v1/social/graph/privacy",
            get(query_handler::get_privacy_setting)
                .patch(action_handler::patch_privacy_setting),
        )
}
