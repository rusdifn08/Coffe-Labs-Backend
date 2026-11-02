mod app;
mod core;
mod modules;

use mimalloc::MiMalloc;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    core::{
        config::AppConfig, db::init_db, middleware::SecurityRateLimiter, security::JwtKeyRing,
        telemetry::TelemetryRegistry, AppState,
    },
    modules::platform::repository::fetch_version_config,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,coffe_labs_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let port = config.port;

    info!("☕ Memulai Coffee Labs Enterprise Rust Axum Backend v2.0.0 (mimalloc)...");

    let pool = init_db(&config.database_url).await?;
    let jwt_keys = JwtKeyRing::from_seed(&config.jwt_seed)?;
    let initial_version = fetch_version_config(&pool).await;
    let version_cache = Arc::new(RwLock::new(initial_version));
    let telemetry = TelemetryRegistry::new();
    let rate_limiter = SecurityRateLimiter::new(
        config.rate_limit_per_second,
        config.rate_limit_burst,
    );

    let state = AppState {
        pool,
        config,
        jwt_keys,
        telemetry,
        rate_limiter,
        version_cache,
    };

    let router = app::build_app(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("🚀 Server berjalan di http://0.0.0.0:{}", port);
    info!("📊 Real-Time APM Dashboard: http://localhost:{}/dashboard", port);
    info!("📄 OpenAPI Specification:  http://localhost:{}/api/v1/openapi.json", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
