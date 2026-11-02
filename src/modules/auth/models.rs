use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub role: String,
    pub status: String,
    pub tier: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub status: String,
    pub tier: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<UserRow> for UserSummary {
    fn from(u: UserRow) -> Self {
        Self {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
            role: u.role,
            status: u.status,
            tier: u.tier,
            avatar_url: u.avatar_url,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct RefreshTokenSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    pub token_hash: String,
    pub device_id: String,
    pub device_name: String,
    pub ip_address: Option<String>,
    pub is_revoked: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
