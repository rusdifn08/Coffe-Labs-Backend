use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub const MAX_IDLE_REGULAR_SEC: i32 = 180;
pub const MAX_IDLE_BREW_OR_AOD_SEC: i32 = 900;
pub const MAX_BATCH_ITEMS: usize = 100;

#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct TelemetryEventInput {
    pub event_id: Uuid,
    pub session_id: Uuid,
    #[validate(length(min = 1, max = 32))]
    pub event_type: String,
    #[validate(length(min = 1, max = 32))]
    pub feature_bucket: String,
    #[validate(length(min = 1, max = 80))]
    pub route_name: String,
    #[serde(default)]
    pub duration_sec: i32,
    #[serde(default)]
    pub is_aod_mode: bool,
    #[serde(default)]
    pub metadata_json: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct TelemetryBatchRequest {
    #[validate(length(max = 20))]
    #[serde(default = "default_platform")]
    pub platform: String,
    #[validate(length(max = 30))]
    #[serde(default)]
    pub app_version: Option<String>,
    #[validate(length(max = 80))]
    #[serde(default)]
    pub device_model: Option<String>,
    #[serde(default)]
    pub active_route: Option<String>,
    #[serde(default)]
    pub active_feature: Option<String>,
    #[serde(default)]
    pub is_brewing: bool,
    #[validate(length(min = 1, max = 100))]
    pub events: Vec<TelemetryEventInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct HeartbeatRequest {
    #[validate(length(min = 1, max = 80))]
    pub active_route: String,
    #[validate(length(min = 1, max = 32))]
    pub feature_bucket: String,
    #[serde(default = "default_platform")]
    pub platform: String,
    #[serde(default)]
    pub app_version: Option<String>,
    #[serde(default)]
    pub device_model: Option<String>,
    #[serde(default)]
    pub is_brewing: bool,
    #[serde(default)]
    pub is_aod_mode: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AccountListQuery {
    pub q: Option<String>,
    pub tier: Option<String>,
    pub risk: Option<String>,
    pub online_only: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchIngestResponse {
    pub accepted_events: usize,
    pub dropped_backpressure: usize,
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeatureBreakdownItem {
    pub feature: String,
    pub label: String,
    pub seconds: i64,
    pub share_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountMonitorSummaryDto {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub account_tier: String,
    pub avatar_url: Option<String>,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub current_active_route: Option<String>,
    pub current_feature_bucket: Option<String>,
    pub is_brewing_now: bool,
    pub platform: String,
    pub app_version: Option<String>,
    pub today_screen_time_sec: i32,
    pub today_aod_time_sec: i32,
    pub total_7d_screen_time_sec: i32,
    pub total_30d_screen_time_sec: i32,
    pub total_lifetime_screen_time_sec: i64,
    pub total_brews_executed: i32,
    pub total_recipes_created: i32,
    pub total_social_posts: i32,
    pub last_brewed_recipe_title: Option<String>,
    pub health_score: i16,
    pub churn_risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedAccountsResponse {
    pub items: Vec<AccountMonitorSummaryDto>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub online_now: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DailyActivityPoint {
    pub metric_date: NaiveDate,
    pub active_screen_time_sec: i32,
    pub aod_kiosk_time_sec: i32,
    pub session_count: i32,
    pub events_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BrewExperimentSummaryDto {
    pub id: Uuid,
    pub recipe_title: String,
    pub brew_method: String,
    pub bean_origin: Option<String>,
    pub brew_duration_sec: i32,
    pub completed: bool,
    pub steps_completed: i32,
    pub rating: Option<i16>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Account360DossierResponse {
    pub account: AccountMonitorSummaryDto,
    pub avg_daily_screen_time_sec: i32,
    pub total_sessions_30d: i32,
    pub total_ai_scans: i32,
    pub feature_breakdown: Vec<FeatureBreakdownItem>,
    pub top_favorite_features: Vec<String>,
    pub daily_timeline: Vec<DailyActivityPoint>,
    pub recent_brew_experiments: Vec<BrewExperimentSummaryDto>,
}

fn default_platform() -> String {
    "android".to_string()
}
