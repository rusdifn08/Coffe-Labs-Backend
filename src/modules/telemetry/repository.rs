use sqlx::{PgPool, Row};
use std::time::Instant;

use crate::modules::telemetry::models::{DatabasePoolMetrics, DomainEntityCounts};

pub async fn probe_database(pool: &PgPool) -> (DatabasePoolMetrics, DomainEntityCounts) {
    let start = Instant::now();
    let row_res = sqlx::query(
        r#"SELECT
            (SELECT COUNT(*) FROM users) AS users,
            (SELECT COUNT(*) FROM auth_refresh_tokens WHERE is_revoked = FALSE) AS active_sessions,
            (SELECT COUNT(*) FROM social_posts) AS social_posts,
            (SELECT COUNT(*) FROM social_comments) AS social_comments,
            (SELECT COUNT(*) FROM user_recipes) AS synced_recipes,
            (SELECT COUNT(*) FROM brew_logs) AS brew_logs,
            (SELECT COUNT(*) FROM curated_beans) AS curated_beans,
            (SELECT COUNT(*) FROM marketplace_orders) AS marketplace_orders,
            (SELECT COUNT(*) FROM feedbacks) AS feedbacks"#,
    )
    .fetch_one(pool)
    .await;

    let ping_ms = ((start.elapsed().as_micros() as f64) / 1000.0 * 100.0).round() / 100.0;

    match row_res {
        Ok(row) => {
            let counts = DomainEntityCounts {
                users: row.try_get("users").unwrap_or(0),
                active_sessions: row.try_get("active_sessions").unwrap_or(0),
                social_posts: row.try_get("social_posts").unwrap_or(0),
                social_comments: row.try_get("social_comments").unwrap_or(0),
                synced_recipes: row.try_get("synced_recipes").unwrap_or(0),
                brew_logs: row.try_get("brew_logs").unwrap_or(0),
                curated_beans: row.try_get("curated_beans").unwrap_or(0),
                marketplace_orders: row.try_get("marketplace_orders").unwrap_or(0),
                feedbacks: row.try_get("feedbacks").unwrap_or(0),
            };
            let db = DatabasePoolMetrics {
                connected: true,
                ping_latency_ms: ping_ms,
                pool_size: pool.size(),
                idle_connections: pool.num_idle(),
                max_connections: 10,
                engine: "NeonDB Serverless PostgreSQL 16 (Rustls)".to_string(),
            };
            (db, counts)
        }
        Err(_) => (
            DatabasePoolMetrics {
                connected: false,
                ping_latency_ms: ping_ms,
                pool_size: pool.size(),
                idle_connections: pool.num_idle(),
                max_connections: 10,
                engine: "NeonDB Serverless PostgreSQL 16 (Reconnecting)".to_string(),
            },
            DomainEntityCounts {
                users: 0,
                active_sessions: 0,
                social_posts: 0,
                social_comments: 0,
                synced_recipes: 0,
                brew_logs: 0,
                curated_beans: 0,
                marketplace_orders: 0,
                feedbacks: 0,
            },
        ),
    }
}
