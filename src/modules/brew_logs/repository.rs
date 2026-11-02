use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::brew_logs::models::{BrewLogRow, MethodFrequency},
};

pub async fn list_logs(
    pool: &PgPool,
    username: &str,
    log_type: &str,
    limit: i64,
) -> AppResult<Vec<BrewLogRow>> {
    let rows = sqlx::query_as::<_, BrewLogRow>(
        r#"SELECT * FROM brew_logs
           WHERE log_type = $1
             AND ($2::text = '' OR username = $2)
           ORDER BY created_at DESC
           LIMIT $3"#,
    )
    .bind(log_type)
    .bind(username)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn insert_log(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    log_type: &str,
    entry_local_id: Option<i64>,
    method: &str,
    bean_name: Option<&str>,
    rating: f64,
    payload: serde_json::Value,
) -> AppResult<BrewLogRow> {
    let row = sqlx::query_as::<_, BrewLogRow>(
        r#"INSERT INTO brew_logs (
            user_id, username, log_type, entry_local_id, method, bean_name, rating, payload
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(log_type)
    .bind(entry_local_id)
    .bind(method)
    .bind(bean_name)
    .bind(rating)
    .bind(payload)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn delete_journal_by_local_id(
    pool: &PgPool,
    username: &str,
    entry_local_id: i64,
) -> AppResult<u64> {
    let res = sqlx::query(
        "DELETE FROM brew_logs WHERE log_type = 'journal' AND entry_local_id = $1 AND ($2::text = '' OR username = $2)",
    )
    .bind(entry_local_id)
    .bind(username)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

pub async fn get_analytics(
    pool: &PgPool,
    username: &str,
) -> AppResult<(i64, i64, f64, Vec<MethodFrequency>)> {
    let total_brews: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM brew_logs WHERE log_type = 'history' AND ($1::text = '' OR username = $1)",
    )
    .bind(username)
    .fetch_one(pool)
    .await?;

    let total_journal: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM brew_logs WHERE log_type = 'journal' AND ($1::text = '' OR username = $1)",
    )
    .bind(username)
    .fetch_one(pool)
    .await?;

    let avg_rating: f64 = sqlx::query_scalar(
        "SELECT COALESCE(AVG(rating)::FLOAT8, 4.8) FROM brew_logs WHERE ($1::text = '' OR username = $1)",
    )
    .bind(username)
    .fetch_one(pool)
    .await?;

    let methods = sqlx::query_as::<_, MethodFrequency>(
        r#"SELECT method, COUNT(*) as count
           FROM brew_logs
           WHERE ($1::text = '' OR username = $1)
           GROUP BY method
           ORDER BY count DESC
           LIMIT 10"#,
    )
    .bind(username)
    .fetch_all(pool)
    .await?;

    Ok((
        total_brews,
        total_journal,
        (avg_rating * 100.0).round() / 100.0,
        methods,
    ))
}
