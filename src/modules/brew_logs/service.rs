use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, security::Claims},
    modules::brew_logs::{
        dto::{BrewAnalyticsResponse, BrewLogQuery},
        models::BrewLogRow,
        repository,
    },
};

pub async fn fetch_brew_logs(
    pool: &PgPool,
    auth: Option<Claims>,
    log_type: &str,
    q: BrewLogQuery,
) -> AppResult<Vec<serde_json::Value>> {
    let username = auth.map(|c| c.username).or(q.username).unwrap_or_default();
    let limit = q.limit.unwrap_or(100).clamp(1, 200);
    let rows = repository::list_logs(pool, &username, log_type, limit).await?;
    Ok(rows.into_iter().map(|r| r.payload).collect())
}

pub async fn record_brew_entry(
    pool: &PgPool,
    auth: Option<Claims>,
    log_type: &str,
    payload: serde_json::Value,
) -> AppResult<BrewLogRow> {
    let user_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let username = auth
        .map(|c| c.username)
        .unwrap_or_else(|| "guest".to_string());
    let local_id = payload.get("id").and_then(|v| v.as_i64());
    let method = payload
        .get("method")
        .or_else(|| payload.get("brewMethod"))
        .or_else(|| payload.get("category"))
        .and_then(|v| v.as_str())
        .unwrap_or("V60")
        .to_string();
    let bean_name = payload
        .get("beanName")
        .or_else(|| payload.get("coffeeName"))
        .or_else(|| payload.get("title"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let rating = payload
        .get("rating")
        .or_else(|| payload.get("overallScore"))
        .and_then(|v| v.as_f64())
        .unwrap_or(4.8);

    repository::insert_log(
        pool,
        user_id,
        &username,
        log_type,
        local_id,
        &method,
        bean_name.as_deref(),
        rating,
        payload,
    )
    .await
}

pub async fn remove_journal_entry(
    pool: &PgPool,
    auth: Option<Claims>,
    local_id: i64,
) -> AppResult<()> {
    let username = auth.map(|c| c.username).unwrap_or_default();
    let _ = repository::delete_journal_by_local_id(pool, &username, local_id).await?;
    Ok(())
}

pub async fn compute_analytics(
    pool: &PgPool,
    auth: Option<Claims>,
    q: BrewLogQuery,
) -> AppResult<BrewAnalyticsResponse> {
    let username = auth.map(|c| c.username).or(q.username).unwrap_or_default();
    let (total_brews, total_journal_entries, average_sensory_rating, method_breakdown) =
        repository::get_analytics(pool, &username).await?;

    Ok(BrewAnalyticsResponse {
        total_brews,
        total_journal_entries,
        average_sensory_rating,
        method_breakdown,
    })
}
