use axum::{
    routing::{get, post},
    Router,
};

use crate::{core::AppState, modules::rbac::handlers};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/upgrade", post(handlers::upgrade_tier))
        .route("/api/v1/rbac/plans", get(handlers::get_plans))
        .route("/api/v1/rbac/subscription", get(handlers::get_my_subscription))
        .route("/api/v1/rbac/upgrade", post(handlers::upgrade_tier))
        .route("/api/v1/rbac/quota/record-brew", post(handlers::record_brew_quota))
        .route("/api/v1/rbac/verify/pro", get(handlers::verify_pro_access))
        .route("/api/v1/rbac/verify/business", get(handlers::verify_business_access))
}
