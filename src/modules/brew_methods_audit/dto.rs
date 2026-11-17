use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AuditRecipeStepInput {
    pub step_order: i32,
    pub title: String,
    pub duration_seconds: i32,
    pub water_added_grams: f32,
}

#[derive(Debug, Deserialize)]
pub struct AuditRecipeRequest {
    pub recipe_id: Option<i32>,
    pub title: String,
    pub method: String,
    pub category: String,
    pub water_temp_celsius: Option<f32>,
    pub coffee_weight_grams: f32,
    pub total_water_weight_grams: f32,
    pub ice_weight_grams: Option<f32>,
    pub total_time_seconds: i32,
    pub steps: Vec<AuditRecipeStepInput>,
}

#[derive(Debug, Serialize)]
pub struct AuditRecipeResponse {
    pub archetype: String,
    pub v60_protected: bool,
    pub is_cold_brew_alarm_only: bool,
    pub is_valid: bool,
    pub step_water_sum_grams: f32,
    pub step_duration_sum_seconds: i32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalculateColdBrewRequest {
    pub brew_mode: String,
    pub coffee_dose_g: f32,
    pub water_ml: f32,
    pub ambient_temp_c: f32,
    pub drip_rate_bpm: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColdBrewCheckpointDto {
    pub offset_sec: i32,
    pub label: String,
    pub instruction: String,
}

#[derive(Debug, Serialize)]
pub struct ColdBrewPlanResponse {
    pub brew_mode: String,
    pub recommended_duration_sec: i32,
    pub recommended_hours: f32,
    pub drip_interval_ms: Option<i32>,
    pub ratio_label: String,
    pub concentrate_yield_ml: f32,
    pub suggested_dilution_water_or_ice_ml: f32,
    pub checkpoints: Vec<ColdBrewCheckpointDto>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleLongAlarmRequest {
    pub recipe_id: Option<i32>,
    pub recipe_title: String,
    pub method_slug: Option<String>,
    pub brew_mode: String,
    pub coffee_dose_g: f32,
    pub water_ml: f32,
    pub ambient_temp_c: Option<f32>,
    pub drip_rate_bpm: Option<i16>,
    pub duration_seconds: i32,
}

#[derive(Debug, Deserialize)]
pub struct SnoozeAlarmRequest {
    pub snooze_minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct LongAlarmDto {
    pub id: Uuid,
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
    pub remaining_seconds: i64,
    pub checkpoints: serde_json::Value,
    pub alarm_status: String,
    pub snooze_count: i16,
}
