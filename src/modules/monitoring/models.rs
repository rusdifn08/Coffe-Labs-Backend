use chrono::{DateTime, NaiveDate, Utc};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

pub const OVERVIEW_CACHE_MAX: u64 = 4;
pub const OVERVIEW_CACHE_TTL_SECS: u64 = 5;
pub const ACCOUNTS_PAGE_CACHE_MAX: u64 = 32;
pub const ACCOUNTS_PAGE_CACHE_TTL_SECS: u64 = 5;
pub const DOSSIER_CACHE_MAX: u64 = 100;
pub const DOSSIER_CACHE_TTL_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiveAccountState {
    pub user_id: Uuid,
    pub current_route: String,
    pub feature_bucket: String,
    pub tier: String,
    pub platform: String,
    pub app_version: String,
    pub is_brewing_now: bool,
    pub last_seen_epoch: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct DossierDbRow {
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
    pub platform: String,
    pub app_version: Option<String>,
    pub today_screen_time_sec: i32,
    pub today_aod_time_sec: i32,
    pub total_7d_screen_time_sec: i32,
    pub total_30d_screen_time_sec: i32,
    pub total_lifetime_screen_time_sec: i64,
    pub avg_daily_screen_time_sec: i32,
    pub total_sessions_30d: i32,
    pub total_brews_executed: i32,
    pub total_recipes_created: i32,
    pub total_social_posts: i32,
    pub total_ai_scans: i32,
    pub feature_time_breakdown_json: serde_json::Value,
    pub last_brewed_recipe_title: Option<String>,
    pub health_score: i16,
    pub churn_risk_level: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct DailyRollupDbRow {
    pub metric_date: NaiveDate,
    pub active_screen_time_sec: i32,
    pub aod_kiosk_time_sec: i32,
    pub session_count: i32,
    pub events_count: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct BrewExperimentDbRow {
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
pub struct FeatureAdoptionStat {
    pub feature_key: String,
    pub label: String,
    pub active_users_7d: u64,
    pub adoption_percent: f64,
    pub total_minutes_7d: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetentionCohortRow {
    pub cohort_date: NaiveDate,
    pub cohort_size: u64,
    pub d1_pct: Option<f64>,
    pub d3_pct: Option<f64>,
    pub d7_pct: Option<f64>,
    pub d14_pct: Option<f64>,
    pub d30_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, FromRow)]
pub struct TopBrewRecipeStat {
    pub recipe_title: String,
    pub brew_method: String,
    pub total_runs: i64,
    pub completed_runs: i64,
    pub completion_rate_pct: f64,
    pub avg_duration_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MonitoringMemoryGuardStats {
    pub mpsc_queue_capacity: usize,
    pub mpsc_pending_batches: usize,
    pub dropped_batches_total: u64,
    pub bitmap_days_in_ram: usize,
    pub live_mem_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlatformOverviewResponse {
    pub online_now: usize,
    pub total_accounts: i64,
    pub trial_accounts: i64,
    pub pro_accounts: i64,
    pub business_accounts: i64,
    pub dau_today: u64,
    pub wau_7d: u64,
    pub mau_30d: u64,
    pub stickiness_pct: f64,
    pub avg_today_screen_time_sec: i32,
    pub total_brews_7d: i64,
    pub at_risk_high_count: i64,
    pub dormant_count: i64,
    pub feature_adoption: Vec<FeatureAdoptionStat>,
    pub cohorts: Vec<RetentionCohortRow>,
    pub top_brew_recipes: Vec<TopBrewRecipeStat>,
    pub ram_guard: MonitoringMemoryGuardStats,
    pub generated_at: DateTime<Utc>,
}

/// Ringkasan non-PII untuk panel publik `/dashboard` sebelum Admin login.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicMonitoringSummary {
    pub online_now: usize,
    pub total_accounts: i64,
    pub dau_today: u64,
    pub wau_7d: u64,
    pub mau_30d: u64,
    pub stickiness_pct: f64,
    pub feature_adoption: Vec<FeatureAdoptionStat>,
    pub top_brew_recipes: Vec<TopBrewRecipeStat>,
    pub ram_guard: MonitoringMemoryGuardStats,
}

#[derive(Clone)]
pub struct MonitoringL1Caches {
    pub overview: Cache<String, Arc<PlatformOverviewResponse>>,
    pub accounts_page: Cache<String, Arc<crate::modules::monitoring::dto::PaginatedAccountsResponse>>,
    pub dossiers: Cache<Uuid, Arc<crate::modules::monitoring::dto::Account360DossierResponse>>,
}

impl Default for MonitoringL1Caches {
    fn default() -> Self {
        Self::new()
    }
}

impl MonitoringL1Caches {
    pub fn new() -> Self {
        Self {
            overview: Cache::builder()
                .max_capacity(OVERVIEW_CACHE_MAX)
                .time_to_live(Duration::from_secs(OVERVIEW_CACHE_TTL_SECS))
                .build(),
            accounts_page: Cache::builder()
                .max_capacity(ACCOUNTS_PAGE_CACHE_MAX)
                .time_to_live(Duration::from_secs(ACCOUNTS_PAGE_CACHE_TTL_SECS))
                .build(),
            dossiers: Cache::builder()
                .max_capacity(DOSSIER_CACHE_MAX)
                .time_to_live(Duration::from_secs(DOSSIER_CACHE_TTL_SECS))
                .build(),
        }
    }

    pub fn invalidate_reads(&self) {
        self.overview.invalidate_all();
        self.accounts_page.invalidate_all();
    }
}
