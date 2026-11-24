pub mod admin_control_schema;
pub mod affiliate_schema;
pub mod app_version_gatekeeper_schema;
pub mod auth_email_reset_schema;
pub mod barista_toolkit_schema;
pub mod beans_radar_schema;
pub mod brew_methods_audit_schema;
pub mod business_schema;
pub mod catalog_schema;
pub mod chat_schema;
pub mod cloud_assets_schema;
pub mod communities_schema;
pub mod core_schema;
pub mod live_brew_schema;
pub mod marketplace_schema;
pub mod monitoring_schema;
pub mod notifications_schema;
pub mod profile_feed_schema;
pub mod qris_billing_schema;
pub mod roasteries_schema;
pub mod social_graph_schema;
pub mod transfer_learning_schema;
pub mod user_cloud_state_schema;

use sqlx::PgPool;
use tracing::warn;

use crate::core::error::AppResult;

async fn exec_batch(pool: &PgPool, module_name: &str, stmts: &[&str]) {
    for (idx, sql) in stmts.iter().enumerate() {
        if let Err(err) = sqlx::query(sql).execute(pool).await {
            warn!(module = module_name, stmt_index = idx, error = %err, "Migration statement warning");
        }
    }
}

pub async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    exec_batch(pool, "core_schema", core_schema::statements()).await;
    exec_batch(pool, "business_schema", business_schema::statements()).await;
    exec_batch(pool, "catalog_schema", catalog_schema::statements()).await;
    exec_batch(pool, "transfer_learning_schema", transfer_learning_schema::statements()).await;
    exec_batch(pool, "chat_schema", chat_schema::statements()).await;
    exec_batch(pool, "profile_feed_schema", profile_feed_schema::statements()).await;
    exec_batch(pool, "social_graph_schema", social_graph_schema::statements()).await;
    exec_batch(pool, "notifications_schema", notifications_schema::statements()).await;
    exec_batch(pool, "communities_schema", communities_schema::statements()).await;
    exec_batch(pool, "monitoring_schema", monitoring_schema::statements()).await;
    exec_batch(pool, "barista_toolkit_schema", barista_toolkit_schema::statements()).await;
    exec_batch(pool, "live_brew_schema", live_brew_schema::statements()).await;
    exec_batch(pool, "brew_methods_audit_schema", brew_methods_audit_schema::statements()).await;
    exec_batch(pool, "roasteries_schema", roasteries_schema::statements()).await;
    exec_batch(pool, "marketplace_schema", marketplace_schema::statements()).await;
    exec_batch(pool, "beans_radar_schema", beans_radar_schema::statements()).await;
    exec_batch(pool, "qris_billing_schema", qris_billing_schema::statements()).await;
    exec_batch(pool, "affiliate_schema", affiliate_schema::statements()).await;
    exec_batch(pool, "admin_control_schema", admin_control_schema::statements()).await;
    exec_batch(pool, "user_cloud_state_schema", user_cloud_state_schema::statements()).await;
    exec_batch(pool, "auth_email_reset_schema", auth_email_reset_schema::statements()).await;
    exec_batch(pool, "app_version_gatekeeper_schema", app_version_gatekeeper_schema::statements()).await;
    exec_batch(pool, "cloud_assets_schema", cloud_assets_schema::statements()).await;
    Ok(())
}
