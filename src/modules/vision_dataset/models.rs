use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VisionClassEntity {
    pub id: Uuid,
    pub class_slug: String,
    pub brand: String,
    pub coffee_name: String,
    pub process: String,
    pub tasting_notes: serde_json::Value,
    pub centroid_vector: Option<Vec<u8>>,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
