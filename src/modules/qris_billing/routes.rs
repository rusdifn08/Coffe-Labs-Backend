use axum::{
    routing::{get, post},
    Router,
};

use crate::core::AppState;
use crate::modules::qris_billing::handlers::{
    admin_reconcile_handler, bridge_ingest_handler, checkout_handler, checkout_ws_handler,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/billing/qris/plans", get(checkout_handler::list_plans))
        .route(
            "/api/v1/billing/qris/checkout",
            post(checkout_handler::create_or_resume_checkout),
        )
        .route(
            "/api/v1/billing/qris/invoices/{id}/status",
            get(checkout_handler::get_invoice_status),
        )
        .route(
            "/api/v1/billing/qris/invoices/{id}/ws",
            get(checkout_ws_handler::invoice_status_ws),
        )
        .route(
            "/api/v1/billing/qris/invoices/{id}/manual-proof",
            post(bridge_ingest_handler::upload_manual_proof),
        )
        .route(
            "/api/v1/billing/qris/bridge-ingest",
            post(bridge_ingest_handler::ingest_livin_bridge_notification),
        )
        .route(
            "/api/v1/billing/qris/admin/unmatched-mutations",
            get(admin_reconcile_handler::list_unmatched_and_recent_mutations),
        )
        .route(
            "/api/v1/billing/qris/admin/pending-invoices",
            get(admin_reconcile_handler::list_pending_and_manual_invoices),
        )
        .route(
            "/api/v1/billing/qris/admin/manual-approve/{invoice_id}",
            post(admin_reconcile_handler::manual_approve_invoice),
        )
        .route(
            "/api/v1/billing/qris/admin/test-dynamic-qris",
            post(admin_reconcile_handler::admin_test_dynamic_qris),
        )
        .route(
            "/api/v1/billing/qris/admin/bridge-info",
            get(admin_reconcile_handler::get_admin_bridge_info),
        )
}
