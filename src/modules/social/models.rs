use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SocialPost {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub author_role: String,
    pub content: String,
    pub recipe_id: Option<i32>,
    pub recipe_snapshot: Option<serde_json::Value>,
    pub likes_count: i32,
    pub comments_count: i32,
    pub copies_count: i32,
    pub tags: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct SocialComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
