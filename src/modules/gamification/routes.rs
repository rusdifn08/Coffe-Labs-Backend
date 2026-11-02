use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::gamification::handlers::{
        claim_domain_action_handler, complete_brew_session_handler, get_badges_catalog_handler,
        get_leaderboard_handler, get_my_profile, start_brew_session_handler,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/gamification/profile", get(get_my_profile))
        .route("/api/v1/gamification/badges", get(get_badges_catalog_handler))
        .route("/api/v1/gamification/leaderboard", get(get_leaderboard_handler))
        .route("/api/v1/gamification/brew-session/start", post(start_brew_session_handler))
        .route("/api/v1/gamification/brew-session/complete", post(complete_brew_session_handler))
        .route("/api/v1/gamification/actions/claim", post(claim_domain_action_handler))
}
