use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::brew_methods_audit::handlers::{catalog_audit_handler, long_alarm_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/brew/methods",
            get(catalog_audit_handler::get_methods_catalog),
        )
        .route(
            "/api/v1/brew/methods/audit",
            post(catalog_audit_handler::post_audit_recipe),
        )
        .route(
            "/api/v1/brew/coldbrew/calculate",
            post(catalog_audit_handler::post_calculate_coldbrew_plan),
        )
        .route(
            "/api/v1/brew/long-alarms",
            get(long_alarm_handler::list_active_long_alarms)
                .post(long_alarm_handler::schedule_long_alarm),
        )
        .route(
            "/api/v1/brew/long-alarms/{alarm_id}/snooze",
            post(long_alarm_handler::snooze_long_alarm),
        )
        .route(
            "/api/v1/brew/long-alarms/{alarm_id}/complete",
            post(long_alarm_handler::complete_long_alarm),
        )
        .route(
            "/api/v1/brew/long-alarms/{alarm_id}",
            delete(long_alarm_handler::cancel_long_alarm),
        )
}
