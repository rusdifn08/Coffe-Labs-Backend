use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{
    core::AppState,
    modules::admin_control::handlers::{
        admin_lock_handler, admin_pricing_handler, public_sync_handler,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Backend Web Control Panel (HTML Dashboard)
        .route("/admin-control", get(public_sync_handler::serve_web_control_panel))
        .route("/admin-control/", get(public_sync_handler::serve_web_control_panel))
        // Public Real-Time Control Sync (REST + Global WebSocket)
        .route(
            "/api/v1/system/control-snapshot",
            get(public_sync_handler::get_control_snapshot),
        )
        .route(
            "/api/v1/system/control-ws",
            get(public_sync_handler::control_ws_upgrade),
        )
        // Admin-Only Feature Card Lock / Unlock Controls
        .route(
            "/api/v1/admin/control/features",
            get(admin_lock_handler::list_admin_features),
        )
        .route(
            "/api/v1/admin/control/features/bulk-lock",
            post(admin_lock_handler::bulk_lock_features),
        )
        .route(
            "/api/v1/admin/control/features/{feature_key}",
            put(admin_lock_handler::update_feature_lock),
        )
        // Admin-Only Dynamic Pro & Business Pricing + Audit Logs
        .route(
            "/api/v1/admin/control/pricing",
            get(admin_pricing_handler::list_admin_pricing),
        )
        .route(
            "/api/v1/admin/control/pricing/{plan_code}",
            put(admin_pricing_handler::update_plan_pricing),
        )
        .route(
            "/api/v1/admin/control/audit-logs",
            get(admin_pricing_handler::list_audit_logs),
        )
}
