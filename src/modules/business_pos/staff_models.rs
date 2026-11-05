use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessStaffMember {
    pub id: Uuid,
    pub store_id: Uuid,
    pub user_id: Option<Uuid>,
    pub full_name: String,
    pub role_title: String,
    #[serde(skip_serializing)]
    pub passcode_pin_hash: String,
    pub can_give_discounts: bool,
    pub can_void_orders: bool,
    pub can_view_reports: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
