mod auth;
mod db;
mod handlers;
mod models;

use axum::{
    extract::FromRef,
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_DATABASE_URL: &str = "postgresql://neondb_owner:npg_pv3TaxJgw0rD@ep-long-term-azhzlfs5-pooler.c-3.ap-southeast-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require";

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub version_cache: Arc<RwLock<models::AppVersionConfig>>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Arc<RwLock<models::AppVersionConfig>> {
    fn from_ref(state: &AppState) -> Self {
        state.version_cache.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,coffe_labs_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 2. Load .env
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    info!("☕ Memulai Coffee Labs Rust Axum Backend Service...");

    // 3. Connect to Database & Auto Migration
    let pool = db::init_db(&database_url).await.map_err(|e| {
        tracing::error!("❌ Gagal terhubung ke database: {}", e);
        e
    })?;

    // Inisialisasi in-memory cache untuk Version Gatekeeper super cepat
    let initial_config: models::AppVersionConfig = match sqlx::query_as::<_, models::AppVersionConfig>(
        "SELECT * FROM app_version_configs WHERE id = 'default'",
    )
    .fetch_one(&pool)
    .await
    {
        Ok(c) => c,
        Err(_) => models::AppVersionConfig {
            id: "default".to_string(),
            platform: "all".to_string(),
            min_base_version: "1.0.0".to_string(),
            latest_base_version: "1.0.0".to_string(),
            min_build_number: 1,
            latest_build_number: 1,
            latest_patch_number: 0,
            is_force_update: false,
            is_patch_required: false,
            release_notes: "Pembaruan stabilitas sistem dan optimasi performa Coffee Labs.".to_string(),
            download_url: Some("https://github.com/rusdi-labs/coffee-hub-mobile/releases/latest".to_string()),
            checksum_sha256: None,
            updated_at: chrono::Utc::now(),
        },
    };

    let version_cache = Arc::new(RwLock::new(initial_config));
    let state = AppState {
        pool,
        version_cache,
    };

    // 4. Setup CORS (Bebas diakses dari emulator Android, Web, Desktop & Mobile)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 5. Router definition
    let app = Router::new()
        // Public health endpoints (support both /health and /api/health)
        .route("/health", get(handlers::health))
        .route("/api/health", get(handlers::health))
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        // The Hybrid 3-Tier Version Gatekeeper endpoint
        .route("/api/v1/app/version-check", get(handlers::version_check))
        // Feedback endpoints (Public & Member)
        .route("/api/feedback", post(handlers::submit_feedback))
        // Social Coffee Community endpoints (Feed, Post, Like, Comment, Copy Recipe)
        .route(
            "/api/v1/social/posts",
            get(handlers::get_social_posts).post(handlers::create_social_post),
        )
        .route("/api/v1/social/posts/:id/like", post(handlers::toggle_like_social_post))
        .route(
            "/api/v1/social/posts/:id/comments",
            get(handlers::get_social_post_comments).post(handlers::add_social_post_comment),
        )
        .route("/api/v1/social/posts/:id/copy-recipe", post(handlers::copy_social_post_recipe))
        // Authenticated user endpoints
        .route("/api/auth/me", get(handlers::get_me))
        // Admin endpoints (User Approval)
        .route("/api/admin/users", get(handlers::admin_get_users))
        .route("/api/admin/users/:id/accept", post(handlers::admin_accept_user))
        .route("/api/admin/users/:id/decline", post(handlers::admin_decline_user))
        .route("/api/admin/users/:id", delete(handlers::admin_delete_user))
        // Admin endpoints (Feedback Management)
        .route("/api/admin/feedbacks", get(handlers::admin_get_feedbacks))
        .route("/api/admin/feedbacks/stats", get(handlers::admin_get_feedback_stats))
        .route("/api/admin/feedbacks/:id/status", post(handlers::admin_update_feedback_status))
        .route("/api/admin/feedbacks/:id", delete(handlers::admin_delete_feedback))
        // Admin endpoints (Version Gatekeeper Configuration)
        .route(
            "/api/admin/app-version",
            get(handlers::admin_get_version_config).post(handlers::admin_update_version_config),
        )
        .layer(cors)
        .with_state(state);

    // 6. Bind listener
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Server berjalan di http://0.0.0.0:{}", port);
    info!("🔐 Admin default: rusdifn08 / 121234aa (Status: Approved)");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
