use axum::{
    routing::{delete, get},
    Router,
};

use crate::{core::AppState, modules::brew_logs::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/brew-logs",
            get(handlers::get_brew_history).post(handlers::save_brew_history),
        )
        .route(
            "/api/brew-logs/journal",
            get(handlers::get_journal_entries).post(handlers::save_journal_entry),
        )
        .route(
            "/api/brew-logs/journal/{id}",
            delete(handlers::delete_journal_entry),
        )
        .route("/api/brew-logs/analytics", get(handlers::get_brew_analytics))
}
