use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeviceTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_installation_id: String,
    pub platform: String,
    pub fcm_token: String,
    pub apns_voip_or_live_activity_token: Option<String>,
    pub app_version: String,
    pub device_model: Option<String>,
    pub is_active: bool,
    pub is_kiosk_aod_active: bool,
    pub failed_delivery_count: i32,
    pub last_active_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationPreferenceRow {
    pub user_id: Uuid,
    pub enable_chat_messages: bool,
    pub enable_follow_follback: bool,
    pub enable_thread_replies: bool,
    pub enable_likes_and_recipe_tries: bool,
    pub enable_live_brew_and_streak: bool,
    pub enable_transactional_billing: bool,
    pub quiet_hours_start_minute: Option<i32>,
    pub quiet_hours_end_minute: Option<i32>,
    pub timezone: String,
    pub updated_at: DateTime<Utc>,
}

impl NotificationPreferenceRow {
    pub fn default_for(user_id: Uuid) -> Self {
        Self {
            user_id,
            enable_chat_messages: true,
            enable_follow_follback: true,
            enable_thread_replies: true,
            enable_likes_and_recipe_tries: true,
            enable_live_brew_and_streak: true,
            enable_transactional_billing: true,
            quiet_hours_start_minute: None,
            quiet_hours_end_minute: None,
            timezone: "Asia/Jakarta".to_string(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationInboxRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub coalesce_group_key: Option<String>,
    pub category: String,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub deep_link_path: String,
    pub action_payload: serde_json::Value,
    pub actors_preview: serde_json::Value,
    pub aggregated_count: i32,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
