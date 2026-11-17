use chrono::{DateTime, Utc};

use crate::modules::monitoring::{
    dto::AccountMonitorSummaryDto,
    models::{DossierDbRow, LiveAccountState},
    services::dossier_aggregator::{classify_churn_risk, compute_health_score},
};

pub fn map_dossier_summary(
    r: DossierDbRow,
    live: Option<&LiveAccountState>,
) -> AccountMonitorSummaryDto {
    let is_online = live.is_some() || r.is_online;
    let last_seen_at = live
        .and_then(|l| DateTime::<Utc>::from_timestamp(l.last_seen_epoch, 0))
        .or(r.last_seen_at);
    let days_idle = last_seen_at.map(|dt| (Utc::now() - dt).num_days().max(0));
    let feat_count = r
        .feature_time_breakdown_json
        .as_object()
        .map(|m| m.len())
        .unwrap_or(0);
    let health_score = if is_online {
        r.health_score.max(compute_health_score(
            0,
            r.total_sessions_30d,
            r.total_brews_executed,
            feat_count,
        ))
    } else {
        compute_health_score(
            days_idle.unwrap_or(30),
            r.total_sessions_30d,
            r.total_brews_executed,
            feat_count,
        )
    };
    let churn_risk_level = if is_online {
        "LOW".to_string()
    } else {
        classify_churn_risk(days_idle, health_score).to_string()
    };

    AccountMonitorSummaryDto {
        user_id: r.user_id,
        username: r.username,
        full_name: r.full_name,
        role: r.role,
        account_tier: live
            .map(|l| l.tier.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or(r.account_tier),
        avatar_url: r.avatar_url,
        is_online,
        last_seen_at,
        current_active_route: live.map(|l| l.current_route.clone()).or(r.current_active_route),
        current_feature_bucket: live
            .map(|l| l.feature_bucket.clone())
            .or(r.current_feature_bucket),
        is_brewing_now: live.is_some_and(|l| l.is_brewing_now),
        platform: live
            .map(|l| l.platform.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or(r.platform),
        app_version: live
            .map(|l| l.app_version.clone())
            .filter(|s| !s.is_empty())
            .or(r.app_version),
        today_screen_time_sec: r.today_screen_time_sec,
        today_aod_time_sec: r.today_aod_time_sec,
        total_7d_screen_time_sec: r.total_7d_screen_time_sec,
        total_30d_screen_time_sec: r.total_30d_screen_time_sec,
        total_lifetime_screen_time_sec: r.total_lifetime_screen_time_sec,
        total_brews_executed: r.total_brews_executed,
        total_recipes_created: r.total_recipes_created,
        total_social_posts: r.total_social_posts,
        last_brewed_recipe_title: r.last_brewed_recipe_title,
        health_score,
        churn_risk_level,
    }
}
