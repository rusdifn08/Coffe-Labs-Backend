use std::collections::HashMap;

use crate::modules::monitoring::dto::{
    FeatureBreakdownItem, MAX_IDLE_BREW_OR_AOD_SEC, MAX_IDLE_REGULAR_SEC,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SanitizedDuration {
    pub active_screen_sec: i32,
    pub aod_kiosk_sec: i32,
}

pub fn normalize_feature_bucket(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "brew_timer" | "brew" | "timer" => "brew_timer",
        "social_coffee" | "social" | "chat" | "feed" => "social_coffee",
        "coffee_club" | "club" | "clubs" | "communities" => "coffee_club",
        "recipe_lab" | "recipes" | "calculator" => "recipe_lab",
        "ai_scanner" | "scanner" | "bean_radar" | "trainer" => "ai_scanner",
        "gamification" | "badges" | "leaderboard" => "gamification",
        "marketplace" | "shop" => "marketplace",
        "business_pos" | "pos" | "kiosk" => "business_pos",
        "aod_kiosk" | "aod" => "aod_kiosk",
        _ => "general",
    }
}

pub fn feature_human_label(bucket: &str) -> &'static str {
    match bucket {
        "brew_timer" => "Brew Timer & Pour Guidance",
        "social_coffee" => "Social Coffee & Chat",
        "coffee_club" => "Coffee Club Community",
        "recipe_lab" => "Recipe Lab & 4:6 Calculator",
        "ai_scanner" => "AI Bean Scanner & Dial-In",
        "gamification" => "Gamification & Badges",
        "marketplace" => "Roastery Marketplace",
        "business_pos" => "Business POS Cashier",
        "aod_kiosk" => "AOD Cafe Kiosk",
        _ => "Core Exploration",
    }
}

/// Memisahkan Active Screen Time vs AOD Kiosk Time dengan Idle Guard (180s normal / 900s seduh & AOD).
pub fn sanitize_event_duration(
    event_type: &str,
    feature_bucket: &str,
    raw_duration_sec: i32,
    is_aod_mode: bool,
) -> SanitizedDuration {
    let bucket = normalize_feature_bucket(feature_bucket);
    let clamped_nonneg = raw_duration_sec.max(0);
    let is_brew_or_aod = is_aod_mode
        || bucket == "brew_timer"
        || bucket == "aod_kiosk"
        || event_type.eq_ignore_ascii_case("BREW_COMPLETE");

    let cap = if is_brew_or_aod {
        MAX_IDLE_BREW_OR_AOD_SEC
    } else {
        MAX_IDLE_REGULAR_SEC
    };
    let effective = clamped_nonneg.min(cap);

    if is_aod_mode || bucket == "aod_kiosk" {
        SanitizedDuration {
            active_screen_sec: 0,
            aod_kiosk_sec: effective,
        }
    } else {
        SanitizedDuration {
            active_screen_sec: effective,
            aod_kiosk_sec: 0,
        }
    }
}

/// Rumus Skor Kesehatan Akun 0..=100 berbasis recency, active days, brew & variasi fitur.
pub fn compute_health_score(
    days_since_last_seen: i64,
    sessions_last_7d: i32,
    brews_last_7d: i32,
    distinct_features_count: usize,
) -> i16 {
    let recency_score = (100 - (days_since_last_seen.max(0) * 10)).clamp(0, 100) as f64;
    let freq_score = (sessions_last_7d.max(0) as f64 / 7.0 * 100.0).clamp(0.0, 100.0);
    let brew_score = (brews_last_7d.max(0) as f64 * 12.5).clamp(0.0, 100.0);
    let breadth_score = (distinct_features_count as f64 * 20.0).clamp(0.0, 100.0);

    let weighted = 0.35 * recency_score
        + 0.30 * freq_score
        + 0.25 * brew_score
        + 0.10 * breadth_score;

    weighted.round().clamp(0.0, 100.0) as i16
}

pub fn classify_churn_risk(days_since_last_seen: Option<i64>, health_score: i16) -> &'static str {
    let days = days_since_last_seen.unwrap_or(99);
    if days >= 14 {
        "DORMANT"
    } else if health_score < 35 || days >= 7 {
        "HIGH"
    } else if health_score < 65 || days >= 3 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

pub fn build_feature_breakdown(raw_json: &serde_json::Value) -> Vec<FeatureBreakdownItem> {
    let mut totals: HashMap<String, i64> = HashMap::new();
    if let Some(obj) = raw_json.as_object() {
        for (k, v) in obj {
            if let Some(secs) = v.as_i64().filter(|&s| s > 0) {
                let normalized = normalize_feature_bucket(k).to_string();
                *totals.entry(normalized).or_insert(0) += secs;
            }
        }
    }

    let sum_sec: i64 = totals.values().sum();
    if sum_sec <= 0 {
        return Vec::new();
    }

    let mut items: Vec<FeatureBreakdownItem> = totals
        .into_iter()
        .map(|(feature, seconds)| {
            let pct = ((seconds as f64 / sum_sec as f64) * 1000.0).round() / 10.0;
            let label = feature_human_label(&feature).to_string();
            FeatureBreakdownItem {
                feature,
                label,
                seconds,
                share_percent: pct,
            }
        })
        .collect();

    items.sort_by(|a, b| b.seconds.cmp(&a.seconds));
    items
}

pub fn top_feature_names(breakdown: &[FeatureBreakdownItem], limit: usize) -> Vec<String> {
    breakdown
        .iter()
        .take(limit)
        .map(|item| item.feature.clone())
        .collect()
}
