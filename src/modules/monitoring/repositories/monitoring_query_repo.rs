use std::collections::HashMap;

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::monitoring::{
    dto::{
        Account360DossierResponse, AccountListQuery, BrewExperimentSummaryDto, DailyActivityPoint,
        PaginatedAccountsResponse,
    },
    error::MonitoringError,
    models::{BrewExperimentDbRow, DailyRollupDbRow, DossierDbRow, LiveAccountState},
    repositories::dossier_mapper::map_dossier_summary,
    services::dossier_aggregator::{build_feature_breakdown, top_feature_names},
};

pub async fn query_accounts_page(
    pool: &PgPool,
    query: &AccountListQuery,
    live_map: &HashMap<Uuid, LiveAccountState>,
) -> Result<PaginatedAccountsResponse, MonitoringError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(25).clamp(1, 50);
    let offset = (page - 1) * per_page;

    let search = query.q.as_deref().unwrap_or("").trim().to_ascii_lowercase();
    let tier = query.tier.as_deref().unwrap_or("").trim().to_ascii_lowercase();
    let risk = query.risk.as_deref().unwrap_or("").trim().to_ascii_uppercase();
    let only_online = query.online_only.unwrap_or(false);
    let live_ids: Vec<Uuid> = live_map.keys().copied().collect();

    let total: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM account_monitoring_dossiers
           WHERE ($1 = '' OR LOWER(username) LIKE '%' || $1 || '%' OR LOWER(full_name) LIKE '%' || $1 || '%')
             AND ($2 = '' OR LOWER(account_tier) = $2)
             AND ($3 = '' OR UPPER(churn_risk_level) = $3)
             AND (NOT $4 OR is_online = TRUE OR user_id = ANY($5))"#,
    )
    .bind(&search)
    .bind(&tier)
    .bind(&risk)
    .bind(only_online)
    .bind(&live_ids)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, DossierDbRow>(
        r#"SELECT user_id, username, full_name, role, account_tier, avatar_url,
                  is_online, last_seen_at, current_active_route, current_feature_bucket,
                  platform, app_version, today_screen_time_sec, today_aod_time_sec,
                  total_7d_screen_time_sec, total_30d_screen_time_sec, total_lifetime_screen_time_sec,
                  avg_daily_screen_time_sec, total_sessions_30d, total_brews_executed,
                  total_recipes_created, total_social_posts, total_ai_scans,
                  feature_time_breakdown_json, last_brewed_recipe_title,
                  health_score, churn_risk_level
           FROM account_monitoring_dossiers
           WHERE ($1 = '' OR LOWER(username) LIKE '%' || $1 || '%' OR LOWER(full_name) LIKE '%' || $1 || '%')
             AND ($2 = '' OR LOWER(account_tier) = $2)
             AND ($3 = '' OR UPPER(churn_risk_level) = $3)
             AND (NOT $4 OR is_online = TRUE OR user_id = ANY($5))
           ORDER BY (is_online OR user_id = ANY($5)) DESC,
                    today_screen_time_sec DESC, last_seen_at DESC NULLS LAST, username ASC
           LIMIT $6 OFFSET $7"#,
    )
    .bind(&search)
    .bind(&tier)
    .bind(&risk)
    .bind(only_online)
    .bind(&live_ids)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let live = live_map.get(&row.user_id);
            map_dossier_summary(row, live)
        })
        .collect();

    Ok(PaginatedAccountsResponse {
        items,
        total,
        page,
        per_page,
        online_now: live_map.len(),
    })
}

pub async fn query_account_360_dossier(
    pool: &PgPool,
    target_user_id: Uuid,
    live: Option<&LiveAccountState>,
) -> Result<Account360DossierResponse, MonitoringError> {
    let row = sqlx::query_as::<_, DossierDbRow>(
        r#"SELECT user_id, username, full_name, role, account_tier, avatar_url,
                  is_online, last_seen_at, current_active_route, current_feature_bucket,
                  platform, app_version, today_screen_time_sec, today_aod_time_sec,
                  total_7d_screen_time_sec, total_30d_screen_time_sec, total_lifetime_screen_time_sec,
                  avg_daily_screen_time_sec, total_sessions_30d, total_brews_executed,
                  total_recipes_created, total_social_posts, total_ai_scans,
                  feature_time_breakdown_json, last_brewed_recipe_title,
                  health_score, churn_risk_level
           FROM account_monitoring_dossiers WHERE user_id = $1"#,
    )
    .bind(target_user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MonitoringError::AccountNotFound)?;

    let cutoff = Utc::now().date_naive() - Duration::days(14);
    let rollups = sqlx::query_as::<_, DailyRollupDbRow>(
        r#"SELECT metric_date, active_screen_time_sec, aod_kiosk_time_sec, session_count, events_count
           FROM account_daily_telemetry_rollups
           WHERE user_id = $1 AND metric_date >= $2 ORDER BY metric_date DESC LIMIT 14"#,
    )
    .bind(target_user_id)
    .bind(cutoff)
    .fetch_all(pool)
    .await?;

    let brew_rows = sqlx::query_as::<_, BrewExperimentDbRow>(
        r#"SELECT id, recipe_title, brew_method, bean_origin, brew_duration_sec,
                  completed, steps_completed, rating, created_at
           FROM account_brew_experiment_logs WHERE user_id = $1 ORDER BY created_at DESC LIMIT 12"#,
    )
    .bind(target_user_id)
    .fetch_all(pool)
    .await?;

    let breakdown = build_feature_breakdown(&row.feature_time_breakdown_json);
    let favorites = top_feature_names(&breakdown, 3);
    let avg_daily = row.avg_daily_screen_time_sec;
    let total_sessions_30d = row.total_sessions_30d;
    let total_ai_scans = row.total_ai_scans;

    Ok(Account360DossierResponse {
        account: map_dossier_summary(row, live),
        avg_daily_screen_time_sec: avg_daily,
        total_sessions_30d,
        total_ai_scans,
        feature_breakdown: breakdown,
        top_favorite_features: favorites,
        daily_timeline: rollups
            .into_iter()
            .map(|d| DailyActivityPoint {
                metric_date: d.metric_date,
                active_screen_time_sec: d.active_screen_time_sec,
                aod_kiosk_time_sec: d.aod_kiosk_time_sec,
                session_count: d.session_count,
                events_count: d.events_count,
            })
            .collect(),
        recent_brew_experiments: brew_rows
            .into_iter()
            .map(|b| BrewExperimentSummaryDto {
                id: b.id,
                recipe_title: b.recipe_title,
                brew_method: b.brew_method,
                bean_origin: b.bean_origin,
                brew_duration_sec: b.brew_duration_sec,
                completed: b.completed,
                steps_completed: b.steps_completed,
                rating: b.rating,
                created_at: b.created_at,
            })
            .collect(),
    })
}
