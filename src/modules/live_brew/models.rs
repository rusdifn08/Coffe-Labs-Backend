use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::modules::live_brew::protocol::LiveBrewStage;

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct BusinessBrewTableRow {
    pub id: Uuid,
    pub store_id: Option<Uuid>,
    pub owner_user_id: Uuid,
    pub table_number: String,
    pub static_address_code: String,
    pub active_brew_session_id: Option<Uuid>,
    pub qr_scans_count: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow)]
pub struct LiveBrewSessionRow {
    pub id: Uuid,
    pub seq_num: i64,
    pub dynamic_address_code: String,
    pub store_id: Option<Uuid>,
    pub table_id: Option<Uuid>,
    pub pos_order_id: Option<Uuid>,
    pub barista_user_id: Uuid,
    pub barista_display_name: String,
    pub customer_user_id: Option<Uuid>,
    pub customer_display_name: String,
    pub recipe_id: Option<Uuid>,
    pub recipe_title: String,
    pub brew_method: String,
    pub bean_name: String,
    pub roastery_name: String,
    pub bean_origin: String,
    pub bean_process: String,
    pub tasting_notes: serde_json::Value,
    pub target_dose_g: f32,
    pub target_water_ml: f32,
    pub target_temp_c: f32,
    pub secondary_temp_c: Option<f32>,
    pub target_duration_sec: i32,
    pub recipe_steps_json: serde_json::Value,
    pub current_stage: LiveBrewStage,
    pub current_step_index: i16,
    pub elapsed_sec: i32,
    pub current_weight_g: f32,
    pub current_flow_rate_ml_s: f32,
    pub current_temp_c: f32,
    pub is_video_stream_active: bool,
    pub webrtc_whep_endpoint: Option<String>,
    pub recorded_telemetry_curve: serde_json::Value,
    pub final_accuracy_score_pct: Option<f32>,
    pub cheers_reactions_count: i32,
    pub viewer_peak_count: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
