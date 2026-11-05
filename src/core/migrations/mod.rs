pub mod business_schema;
pub mod catalog_schema;
pub mod core_schema;

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

    Ok(())
}
