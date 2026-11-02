use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct UserRecipeRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub recipe_local_id: i64,
    pub title: String,
    pub method: String,
    pub category: String,
    pub is_favorite: bool,
    pub recipe_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
