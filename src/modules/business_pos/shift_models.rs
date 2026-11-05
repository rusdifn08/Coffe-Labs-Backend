use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessCashierShift {
    pub id: Uuid,
    pub store_id: Uuid,
    pub cashier_user_id: Uuid,
    pub cashier_name: String,
    pub shift_name: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub opening_cash_cents: i64,
    pub expected_cash_cents: i64,
    pub actual_cash_cents: Option<i64>,
    pub cash_difference_cents: Option<i64>,
    pub total_orders_count: i32,
    pub total_sales_cents: i64,
    pub status: String, // 'open', 'closed'
    pub notes: Option<String>,
}
