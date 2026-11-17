use std::collections::HashMap;

use sqlx::{FromRow, PgPool};

use crate::modules::monitoring::{error::MonitoringError, models::TopBrewRecipeStat};

#[derive(FromRow)]
pub struct OverviewDbAggRow {
    pub total_accounts: i64,
    pub trial_accounts: i64,
    pub pro_accounts: i64,
    pub business_accounts: i64,
    pub avg_today_screen_time_sec: i32,
    pub at_risk_high_count: i64,
    pub dormant_count: i64,
}

pub async fn query_overview_aggregates(pool: &PgPool) -> Result<OverviewDbAggRow, MonitoringError> {
    let row = sqlx::query_as::<_, OverviewDbAggRow>(
        r#"SELECT
               COUNT(*)::bigint AS total_accounts,
               COUNT(*) FILTER (WHERE LOWER(account_tier) = 'trial')::bigint AS trial_accounts,
               COUNT(*) FILTER (WHERE LOWER(account_tier) = 'pro')::bigint AS pro_accounts,
               COUNT(*) FILTER (WHERE LOWER(account_tier) = 'business')::bigint AS business_accounts,
               COALESCE(AVG(today_screen_time_sec)::int, 0) AS avg_today_screen_time_sec,
               COUNT(*) FILTER (WHERE churn_risk_level = 'HIGH')::bigint AS at_risk_high_count,
               COUNT(*) FILTER (WHERE churn_risk_level = 'DORMANT')::bigint AS dormant_count
           FROM account_monitoring_dossiers"#,
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn query_top_brew_recipes(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<TopBrewRecipeStat>, MonitoringError> {
    let rows = sqlx::query_as::<_, TopBrewRecipeStat>(
        r#"SELECT recipe_title,
                  MAX(brew_method)::varchar AS brew_method,
                  COUNT(*)::bigint AS total_runs,
                  COUNT(*) FILTER (WHERE completed)::bigint AS completed_runs,
                  ROUND(
                      (COUNT(*) FILTER (WHERE completed)::numeric / GREATEST(COUNT(*), 1)::numeric) * 100.0,
                      1
                  )::float8 AS completion_rate_pct,
                  ROUND(COALESCE(AVG(brew_duration_sec), 0)::numeric, 1)::float8 AS avg_duration_sec
           FROM account_brew_experiment_logs
           WHERE created_at >= NOW() - INTERVAL '30 days'
           GROUP BY recipe_title
           ORDER BY total_runs DESC, completed_runs DESC
           LIMIT $1"#,
    )
    .bind(limit.clamp(1, 20))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn query_feature_minutes_7d(
    pool: &PgPool,
) -> Result<(HashMap<String, i64>, i64), MonitoringError> {
    let brews_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM account_brew_experiment_logs WHERE created_at >= NOW() - INTERVAL '7 days'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"SELECT kv.key, COALESCE(SUM((kv.value)::bigint) / 60, 0)::bigint AS mins
           FROM account_daily_telemetry_rollups r,
                jsonb_each_text(r.feature_seconds_json) AS kv(key, value)
           WHERE r.metric_date >= CURRENT_DATE - 7
           GROUP BY kv.key"#,
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    Ok((rows.into_iter().collect(), brews_7d))
}
