use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct AppVersionConfig {
    pub id: String,
    pub platform: String,
    pub min_base_version: String,
    pub latest_base_version: String,
    pub min_build_number: i32,
    pub latest_build_number: i32,
    pub latest_patch_number: i32,
    pub is_force_update: bool,
    pub is_patch_required: bool,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl AppVersionConfig {
    pub fn fallback_default() -> Self {
        Self {
            id: "default".to_string(),
            platform: "all".to_string(),
            min_base_version: "2.0.0".to_string(),
            latest_base_version: "2.0.0".to_string(),
            min_build_number: 2,
            latest_build_number: 2,
            latest_patch_number: 0,
            is_force_update: false,
            is_patch_required: false,
            release_notes: "Pembaruan stabilitas sistem dan optimasi performa Coffee Labs.".to_string(),
            download_url: Some("https://github.com/rusdi-labs/coffee-hub-mobile/releases/latest".to_string()),
            checksum_sha256: None,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpdateAction {
    None,
    SilentOtaPatch,
    ForceOtaPatch,
    FlexibleStoreUpdate,
    ForceStoreUpdate,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct Feedback {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub user_role: String,
    pub category: String,
    pub feature_affected: String,
    pub title: String,
    pub description: String,
    pub rating: Option<i32>,
    pub app_version: Option<String>,
    pub device_info: Option<String>,
    pub status: String,
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeedbackStats {
    pub total: i64,
    pub pending: i64,
    pub in_progress: i64,
    pub resolved: i64,
    pub bug_reports: i64,
    pub feature_requests: i64,
    pub average_rating: f64,
}
