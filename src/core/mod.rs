pub mod config;
pub mod db;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod migrations;
pub mod pagination;
pub mod security;
pub mod seeds;
pub mod telemetry;

use axum::extract::FromRef;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    core::{
        config::AppConfig, middleware::SecurityRateLimiter, security::JwtKeyRing,
        telemetry::TelemetryRegistry,
    },
    modules::platform::models::AppVersionConfig,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
    pub jwt_keys: JwtKeyRing,
    pub telemetry: TelemetryRegistry,
    pub rate_limiter: SecurityRateLimiter,
    pub version_cache: Arc<RwLock<AppVersionConfig>>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for JwtKeyRing {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_keys.clone()
    }
}

impl FromRef<AppState> for TelemetryRegistry {
    fn from_ref(state: &AppState) -> Self {
        state.telemetry.clone()
    }
}

impl FromRef<AppState> for Arc<RwLock<AppVersionConfig>> {
    fn from_ref(state: &AppState) -> Self {
        state.version_cache.clone()
    }
}
