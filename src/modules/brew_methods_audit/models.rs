use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrewExecutionArchetype {
    PercolationPourOver,
    FlashChillIcedFilter,
    SteepAndReleaseImmersion,
    SubSecondPressureEspresso,
    ThermalCuePhaseBrewer,
    LongHorizonPersistentAlarm,
}

impl BrewExecutionArchetype {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PercolationPourOver => "PERCOLATION_POUR_OVER",
            Self::FlashChillIcedFilter => "FLASH_CHILL_ICED_FILTER",
            Self::SteepAndReleaseImmersion => "STEEP_AND_RELEASE_IMMERSION",
            Self::SubSecondPressureEspresso => "SUB_SECOND_PRESSURE_ESPRESSO",
            Self::ThermalCuePhaseBrewer => "THERMAL_CUE_PHASE_BREWER",
            Self::LongHorizonPersistentAlarm => "LONG_HORIZON_PERSISTENT_ALARM",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct BrewMethodCatalogRow {
    pub id: i32,
    pub slug: String,
    pub display_name: String,
    pub display_name_id: String,
    pub archetype: String,
    pub icon_asset_path: String,
    pub filter_key: String,
    pub is_background_alarm_mode: bool,
    pub default_min_duration_sec: i32,
    pub default_max_duration_sec: i32,
    pub sort_order: i16,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct LongBrewAlarmRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recipe_id: Option<i32>,
    pub recipe_title: String,
    pub method_slug: String,
    pub brew_mode: String,
    pub coffee_dose_g: f32,
    pub water_ml: f32,
    pub ambient_temp_c: f32,
    pub drip_rate_bpm: Option<i16>,
    pub started_at: DateTime<Utc>,
    pub target_finish_at: DateTime<Utc>,
    pub total_duration_sec: i32,
    pub checkpoints_json: serde_json::Value,
    pub alarm_status: String,
    pub snooze_count: i16,
    pub last_push_sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
