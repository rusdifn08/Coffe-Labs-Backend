use axum::{
    routing::{delete, get, patch, post},
    Router,
};

use crate::core::AppState;
use crate::modules::business_pos::{
    analytics_handlers, handlers, inventory_handlers, kiosk_handlers, shift_handlers,
    staff_handlers, table_handlers,
};

pub fn router() -> Router<AppState> {
    Router::new()
        // Core Business Store & Menu Catalog
        .route("/api/v1/business/config", get(handlers::get_config).put(handlers::update_config))
        .route("/api/v1/business/menu", get(handlers::list_menu).post(handlers::create_menu_item))
        .route("/api/v1/business/menu/{id}", patch(handlers::update_menu_item).delete(handlers::delete_menu_item))
        .route("/api/v1/business/menu/upload-validate", post(handlers::upload_validate_asset))
        .route("/api/v1/business/pos/orders", post(handlers::create_pos_order))
        .route("/api/v1/business/verify-pin", post(handlers::verify_pin))
        // Kiosk Hardware Locks & Telemetry
        .route("/api/v1/business/kiosk/heartbeat", post(kiosk_handlers::kiosk_heartbeat))
        .route("/api/v1/business/kiosk/emergency-unlock", post(kiosk_handlers::emergency_unlock))
        .route("/api/v1/business/kiosk/recovery-pin", get(kiosk_handlers::get_recovery_pin))
        // Multi-Tenant Inventory & Stock Tracking
        .route("/api/v1/business/inventory", get(inventory_handlers::list_inventory).post(inventory_handlers::create_inventory_item))
        .route("/api/v1/business/inventory/{id}/stock", patch(inventory_handlers::adjust_stock))
        .route("/api/v1/business/inventory/{id}", delete(inventory_handlers::delete_inventory_item))
        // Cashier Shifts & Cash Reconciliation
        .route("/api/v1/business/shifts/current", get(shift_handlers::get_current_shift))
        .route("/api/v1/business/shifts/open", post(shift_handlers::open_shift))
        .route("/api/v1/business/shifts/{id}/close", post(shift_handlers::close_shift))
        .route("/api/v1/business/shifts/history", get(shift_handlers::list_shift_history))
        // Dining Tables & Seating Management
        .route("/api/v1/business/tables", get(table_handlers::list_tables).post(table_handlers::create_table))
        .route("/api/v1/business/tables/{id}/status", patch(table_handlers::update_table_status))
        .route("/api/v1/business/tables/{id}", delete(table_handlers::delete_table))
        // Multi-Barista Staff & Permissions
        .route("/api/v1/business/staff", get(staff_handlers::list_staff).post(staff_handlers::create_staff))
        .route("/api/v1/business/staff/{id}", patch(staff_handlers::update_staff).delete(staff_handlers::delete_staff))
        // High-Performance Store Analytics Summary
        .route("/api/v1/business/analytics/summary", get(analytics_handlers::get_analytics_summary))
}
