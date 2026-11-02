use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AccountTier {
    Trial,
    Pro,
    Business,
}

impl AccountTier {
    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "business" | "plan_coffe_labs_business" => Self::Business,
            "pro" | "plan_coffe_labs_pro" => Self::Pro,
            _ => Self::Trial,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trial => "trial",
            Self::Pro => "pro",
            Self::Business => "business",
        }
    }

    pub fn plan_id(&self) -> &'static str {
        match self {
            Self::Trial => "plan_trial",
            Self::Pro => "plan_coffe_labs_pro",
            Self::Business => "plan_coffe_labs_business",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    BasicBrewTimer,
    UnlimitedBrews,
    FullChampionRecipes,
    ZeroAds,
    AiDialInLab,
    GrinderMatrix,
    WaterAndTdsLab,
    MultiBaristaSync,
    ThermalReceiptPrint,
    SensoryExportPdfCsv,
    BeanRadarPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRbacState {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub tier: String,
    pub trial_brews_count: i32,
    pub trial_locked_until: Option<DateTime<Utc>>,
    pub subscription_plan_id: Option<String>,
    pub subscription_payment_method: Option<String>,
    pub subscription_activated_at: Option<DateTime<Utc>>,
}
