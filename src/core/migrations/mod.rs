pub mod business_schema;
pub mod catalog_schema;
pub mod chat_schema;
pub mod communities_schema;
pub mod core_schema;
pub mod notifications_schema;
pub mod profile_feed_schema;
pub mod social_graph_schema;
pub mod transfer_learning_schema;

use sqlx::PgPool;
use crate::core::error::AppResult;

pub async fn run_migrations(pool: &PgPool) -> AppResult<()> {
    for sql in core_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in business_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in catalog_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in transfer_learning_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in chat_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in profile_feed_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in social_graph_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in notifications_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    for sql in communities_schema::statements() {
        let _ = sqlx::query(sql).execute(pool).await;
    }

    Ok(())
}
