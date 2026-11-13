use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::communities::handlers::{club_handler, invite_handler, vault_feed_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/communities/discover",
            get(club_handler::discover_communities),
        )
        .route(
            "/api/v1/communities",
            post(club_handler::create_community),
        )
        .route(
            "/api/v1/communities/join-by-secret",
            post(invite_handler::join_by_secret),
        )
        .route(
            "/api/v1/communities/join-by-link/{token}",
            post(invite_handler::join_by_token),
        )
        .route(
            "/api/v1/communities/{id}",
            get(club_handler::get_workspace),
        )
        .route(
            "/api/v1/communities/{id}/join",
            post(club_handler::join_public_club),
        )
        .route(
            "/api/v1/communities/{id}/leaderboard",
            get(club_handler::get_leaderboard),
        )
        .route(
            "/api/v1/communities/{id}/invites",
            get(invite_handler::list_invites).post(invite_handler::create_invite),
        )
        .route(
            "/api/v1/communities/{id}/invites/{invite_id}",
            delete(invite_handler::revoke_invite),
        )
        .route(
            "/api/v1/communities/{id}/feed",
            get(vault_feed_handler::get_club_feed).post(vault_feed_handler::create_club_post),
        )
        .route(
            "/api/v1/communities/{id}/recipe-vault",
            get(vault_feed_handler::get_club_recipe_vault),
        )
}
