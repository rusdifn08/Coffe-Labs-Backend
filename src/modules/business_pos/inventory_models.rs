use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessInventoryItem {
    pub id: Uuid,
    pub store_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub stock_quantity: f64,
    pub unit: String,
    pub min_reorder_level: f64,
    pub cost_per_unit_cents: i64,
    pub supplier_name: Option<String>,
    pub last_restocked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
