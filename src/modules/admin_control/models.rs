use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct ControlMetaRow {
    pub config_version: i64,
    pub global_maintenance_mode: bool,
    pub global_maintenance_message: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct FeatureControlRow {
    pub feature_key: String,
    pub display_name: String,
    pub group_section: String,
    pub route_prefix: String,
    pub api_path_prefixes: Value,
    pub display_order: i16,
    pub is_locked: bool,
    pub lock_status: String,
    pub custom_badge_label: Option<String>,
    pub lock_title: String,
    pub lock_message: String,
    pub estimated_unlock_at: Option<DateTime<Utc>>,
    pub allow_admin_bypass: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct DynamicPlanRow {
    pub plan_code: String,
    pub target_tier: String,
    pub display_name: String,
    pub duration_days: i32,
    pub base_price_idr: i64,
    pub original_price_idr: Option<i64>,
    pub promo_badge_text: Option<String>,
    pub is_highlighted: bool,
    pub features_json: Value,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AuditLogRow {
    pub id: Uuid,
    pub admin_user_id: Uuid,
    pub admin_email: String,
    pub action_category: String,
    pub target_key: String,
    pub previous_state_json: Value,
    pub new_state_json: Value,
    pub reason_note: Option<String>,
    pub config_version_after: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AffiliateControlRow {
    pub is_program_active: bool,
    pub buyer_discount_bps: i32,
    pub affiliate_commission_bps: i32,
    pub allow_free_tier_affiliates: bool,
    pub min_withdrawal_idr: i64,
}
