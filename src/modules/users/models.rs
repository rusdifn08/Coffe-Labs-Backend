use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct BaristaProfile {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub status: String,
    pub tier: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub equipment_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AdminUserStats {
    pub total_users: i64,
    pub new_registrations_7d: i64,
    pub active_users: i64,
    pub pending_or_declined: i64,
}
