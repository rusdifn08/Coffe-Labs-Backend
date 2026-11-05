use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessDiningTable {
    pub id: Uuid,
    pub store_id: Uuid,
    pub table_number: String,
    pub section: String,
    pub capacity: i32,
    pub status: String, // 'available', 'occupied', 'reserved', 'cleaning'
    pub active_order_id: Option<Uuid>,
    pub qr_code_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
