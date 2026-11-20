use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::modules::admin_control::models::{AuditLogRow, DynamicPlanRow, FeatureControlRow};

pub const VALID_LOCK_STATUSES: &[&str] = &["UNLOCKED", "MAINTENANCE", "COMING_SOON", "UPDATED"];

pub fn normalize_lock_status(raw: &str, is_locked: bool) -> Option<String> {
    if !is_locked {
        return Some("UNLOCKED".to_string());
    }
    let up = raw.trim().to_ascii_uppercase();
    if up == "UNLOCKED" || up.is_empty() {
        return Some("MAINTENANCE".to_string());
    }
    VALID_LOCK_STATUSES
        .iter()
        .find(|s| **s == up)
        .map(|s| (*s).to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureControlDto {
    pub feature_key: String,
    pub display_name: String,
    pub group_section: String,
    pub route_prefix: String,
    pub api_path_prefixes: Vec<String>,
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

fn json_str_vec(val: &Value) -> Vec<String> {
    val.as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(ToOwned::to_owned)).collect())
        .unwrap_or_default()
}

impl From<FeatureControlRow> for FeatureControlDto {
    fn from(r: FeatureControlRow) -> Self {
        Self {
            feature_key: r.feature_key,
            display_name: r.display_name,
            group_section: r.group_section,
            route_prefix: r.route_prefix,
            api_path_prefixes: json_str_vec(&r.api_path_prefixes),
            display_order: r.display_order,
            is_locked: r.is_locked,
            lock_status: r.lock_status,
            custom_badge_label: r.custom_badge_label,
            lock_title: r.lock_title,
            lock_message: r.lock_message,
            estimated_unlock_at: r.estimated_unlock_at,
            allow_admin_bypass: r.allow_admin_bypass,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicPlanPricingDto {
    pub plan_code: String,
    pub target_tier: String,
    pub display_name: String,
    pub duration_days: i32,
    pub base_price_idr: i64,
    pub original_price_idr: Option<i64>,
    pub promo_badge_text: Option<String>,
    pub is_highlighted: bool,
    pub features: Vec<String>,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<DynamicPlanRow> for DynamicPlanPricingDto {
    fn from(r: DynamicPlanRow) -> Self {
        Self {
            plan_code: r.plan_code,
            target_tier: r.target_tier,
            display_name: r.display_name,
            duration_days: r.duration_days,
            base_price_idr: r.base_price_idr,
            original_price_idr: r.original_price_idr,
            promo_badge_text: r.promo_badge_text,
            is_highlighted: r.is_highlighted,
            features: json_str_vec(&r.features_json),
            is_active: r.is_active,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSnapshotDto {
    pub config_version: i64,
    pub global_maintenance_mode: bool,
    pub global_maintenance_message: Option<String>,
    pub connected_clients: usize,
    pub features: Vec<FeatureControlDto>,
    pub plans: Vec<DynamicPlanPricingDto>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateFeatureLockDto {
    pub is_locked: bool,
    #[serde(default)]
    pub lock_status: String,
    pub custom_badge_label: Option<String>,
    #[serde(default)]
    pub lock_title: String,
    #[serde(default)]
    pub lock_message: String,
    pub estimated_unlock_at: Option<DateTime<Utc>>,
    pub allow_admin_bypass: Option<bool>,
    pub reason_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BulkFeatureLockDto {
    pub feature_keys: Option<Vec<String>>,
    pub is_locked: bool,
    #[serde(default)]
    pub lock_status: String,
    pub custom_badge_label: Option<String>,
    #[serde(default)]
    pub lock_title: String,
    #[serde(default)]
    pub lock_message: String,
    pub estimated_unlock_at: Option<DateTime<Utc>>,
    pub allow_admin_bypass: Option<bool>,
    pub reason_note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePlanPriceDto {
    pub display_name: Option<String>,
    pub base_price_idr: i64,
    pub original_price_idr: Option<i64>,
    pub promo_badge_text: Option<String>,
    pub duration_days: i32,
    pub is_active: Option<bool>,
    pub is_highlighted: Option<bool>,
    pub reason_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditLogDto {
    pub id: Uuid,
    pub admin_user_id: Uuid,
    pub admin_email: String,
    pub action_category: String,
    pub target_key: String,
    pub previous_state: Value,
    pub new_state: Value,
    pub reason_note: Option<String>,
    pub config_version_after: i64,
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogRow> for AuditLogDto {
    fn from(r: AuditLogRow) -> Self {
        Self {
            id: r.id,
            admin_user_id: r.admin_user_id,
            admin_email: r.admin_email,
            action_category: r.action_category,
            target_key: r.target_key,
            previous_state: r.previous_state_json,
            new_state: r.new_state_json,
            reason_note: r.reason_note,
            config_version_after: r.config_version_after,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlWsFrame {
    SnapshotSynced { snapshot: ControlSnapshotDto },
    Heartbeat { config_version: i64, connected_clients: usize },
}
