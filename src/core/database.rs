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

/// Inisialisasi PgPool teroptimasi untuk Serverless NeonDB PostgreSQL Free Tier.
pub async fn init_db(database_url: &str) -> AppResult<PgPool> {
    let clean_url = sanitize_db_url(database_url);
    info!("🔌 Menghubungkan ke Serverless NeonDB PostgreSQL (max_connections=10, min=1)...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Some(Duration::from_secs(300)))
        .max_lifetime(Some(Duration::from_secs(600)))
        .test_before_acquire(true)
        .connect(&clean_url)
        .await?;

    info!("✅ Koneksi NeonDB berhasil! Menjalankan migrasi skema & indeks...");
    migrations::run_migrations(&pool).await?;
    crate::modules::gamification::services::badges_seed::init_gamification_schema_and_badges(&pool).await?;
    seeds::run_seeds(&pool).await?;
    crate::modules::auth::repository::preload_users_cache(&pool).await;
    crate::modules::gamification::services::streak_service::spawn_gamification_cron_jobs(pool.clone()).await;

    Ok(pool)
}
