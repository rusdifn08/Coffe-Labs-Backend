use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, ToSchema)]
pub struct CuratedBeanRow {
    pub id: String,
    pub name: String,
    pub roastery_name: String,
    pub roastery_city: String,
    pub origin: String,
    pub variety: String,
    pub process: String,
    pub roast_level: String,
    pub tasting_notes: String,
    pub price_100g: i32,
    pub price_200g: i32,
    pub rating: f64,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct DialInPourStep {
    pub step_order: i32,
    pub title: String,
    pub water_added_grams: f64,
    pub cumulative_water_grams: f64,
    pub duration_seconds: i32,
    pub instruction: String,
}
