use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::monitoring::handlers::{accounts_monitor_handler, analytics_handler, ingest_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/monitoring/ingest-batch", post(ingest_handler::ingest_batch))
        .route("/api/v1/monitoring/heartbeat", post(ingest_handler::heartbeat))
        .route("/api/v1/monitoring/overview", get(analytics_handler::get_monitoring_overview))
        .route("/api/v1/monitoring/accounts", get(accounts_monitor_handler::list_monitored_accounts))
        .route(
            "/api/v1/monitoring/accounts/{user_id}/dossier",
            get(accounts_monitor_handler::get_account_dossier),
        )
}
