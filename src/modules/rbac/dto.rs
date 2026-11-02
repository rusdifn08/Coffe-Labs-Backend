use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::rbac::models::{AccountTier, PermissionScope};

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpgradeTierRequest {
    #[validate(length(min = 2, max = 60, message = "Plan ID diperlukan."))]
    pub plan_id: String,
    pub payment_method: Option<String>,
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct SubscriptionPlanDto {
    pub id: String,
    pub tier: AccountTier,
    pub title: String,
    pub subtitle: String,
    pub price_label: String,
    pub price_amount: i32,
    pub billing_period: String,
    pub badge_label: String,
    pub is_recommended: bool,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct QuotaStatusResponse {
    pub tier: AccountTier,
    pub plan_id: String,
    pub is_pro_or_business: bool,
    pub is_business: bool,
    pub max_trial_brews: i32,
    pub max_trial_recipes: i32,
    pub current_trial_brews: i32,
    pub remaining_trial_brews: i32,
    pub is_brew_locked: bool,
    pub trial_locked_until: Option<DateTime<Utc>>,
    pub permissions: Vec<PermissionScope>,
    pub activated_at: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
}
