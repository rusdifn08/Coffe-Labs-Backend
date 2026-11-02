use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct BrewLogRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub log_type: String,
    pub entry_local_id: Option<i64>,
    pub method: String,
    pub bean_name: Option<String>,
    pub rating: Option<f64>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct MethodFrequency {
    pub method: String,
    pub count: i64,
}
