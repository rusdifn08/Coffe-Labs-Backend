use secrecy::{ExposeSecret, SecretString};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing::info;

use crate::core::{error::AppResult, migrations, seeds};

pub fn sanitize_db_url(raw: &str) -> String {
    let mut cleaned = raw.to_string();
    cleaned = cleaned.replace("&channel_binding=require", "");
    cleaned = cleaned.replace("channel_binding=require&", "");
    cleaned = cleaned.replace("?channel_binding=require", "");
    if !cleaned.contains("sslmode=") {
        if cleaned.contains('?') {
            cleaned.push_str("&sslmode=require");
        } else {
            cleaned.push_str("?sslmode=require");
        }
    }
    cleaned
}

pub async fn init_db(database_url: &SecretString) -> AppResult<PgPool> {
    let clean_url = sanitize_db_url(database_url.expose_secret());
    info!("🔌 Menghubungkan ke Serverless NeonDB PostgreSQL (Optimized Pool)...");

    let pool = PgPoolOptions::new()
        .max_connections(15)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(12))
        .idle_timeout(Some(Duration::from_secs(300)))
        .max_lifetime(Some(Duration::from_secs(1800)))
        .test_before_acquire(false)
        .connect(&clean_url)
        .await?;

    info!("✅ Koneksi NeonDB berhasil! Menjalankan migrasi skema & indeks...");
    migrations::run_migrations(&pool).await?;
    seeds::run_seeds(&pool).await?;
    crate::modules::auth::repository::preload_users_cache(&pool).await;

    Ok(pool)
}
