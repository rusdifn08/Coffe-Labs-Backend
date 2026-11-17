use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::modules::live_brew::protocol::LiveBrewStage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchupCurvePoint {
    pub t_ms: u32,
    pub w_g: f32,
    pub flow: f32,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBrewTableRequest {
    #[validate(length(min = 1, max = 30))]
    pub table_number: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct BindTableSessionRequest {
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateLiveBrewSessionRequest {
    pub table_id: Option<Uuid>,
    pub pos_order_id: Option<Uuid>,
    pub customer_display_name: Option<String>,
    pub recipe_id: Option<Uuid>,
    #[validate(length(min = 1, max = 140))]
    pub recipe_title: String,
    pub brew_method: Option<String>,
    pub bean_name: Option<String>,
    pub roastery_name: Option<String>,
    pub bean_origin: Option<String>,
    pub bean_process: Option<String>,
    pub tasting_notes: Option<Vec<String>>,
    pub target_dose_g: Option<f32>,
    pub target_water_ml: Option<f32>,
    pub target_temp_c: Option<f32>,
    pub secondary_temp_c: Option<f32>,
    pub target_duration_sec: Option<i32>,
    pub recipe_steps: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrewTableDto {
    pub id: Uuid,
    pub table_number: String,
    pub static_address_code: String,
    pub livebrew_url: String,
    pub deep_link_url: String,
    pub active_brew_session_id: Option<Uuid>,
    pub qr_scans_count: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveBrewSnapshotDto {
    pub session_id: Uuid,
    pub dynamic_address_code: String,
    pub livebrew_url: String,
    pub deep_link_url: String,
    pub table_number: Option<String>,
    pub static_address_code: Option<String>,
    pub store_name: String,
    pub barista_display_name: String,
    pub customer_display_name: String,
    pub recipe_id: Option<Uuid>,
    pub recipe_title: String,
    pub brew_method: String,
    pub bean_name: String,
    pub roastery_name: String,
    pub bean_origin: String,
    pub bean_process: String,
    pub tasting_notes: Vec<String>,
    pub target_dose_g: f32,
    pub target_water_ml: f32,
    pub target_temp_c: f32,
    pub secondary_temp_c: Option<f32>,
    pub target_duration_sec: i32,
    pub recipe_steps: serde_json::Value,
    pub current_stage: LiveBrewStage,
    pub stage_label_id: String,
    pub current_step_index: i16,
    pub elapsed_sec: i32,
    pub current_weight_g: f32,
    pub current_flow_rate_ml_s: f32,
    pub current_temp_c: f32,
    pub is_video_stream_active: bool,
    pub webrtc_whep_endpoint: Option<String>,
    pub cheers_reactions_count: i32,
    pub final_accuracy_score_pct: Option<f32>,
    pub catchup_curve_points: Vec<CatchupCurvePoint>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigitalCupPassportDto {
    pub session_id: Uuid,
    pub dynamic_address_code: String,
    pub recipe_title: String,
    pub bean_name: String,
    pub roastery_name: String,
    pub brew_method: String,
    pub tasting_notes: Vec<String>,
    pub final_weight_g: f32,
    pub total_duration_sec: i32,
    pub accuracy_score_pct: f32,
    pub cheers_reactions_count: i32,
    pub livebrew_url: String,
    pub deep_link_url: String,
    pub apk_download_url: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct SendCheerRequest {
    #[validate(length(min = 1, max = 16))]
    pub cheer_kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolveAddressResponse {
    pub resolution_kind: String, // "SESSION" or "TABLE_WAITING"
    pub address_code: String,
    pub session: Option<LiveBrewSnapshotDto>,
    pub waiting: Option<TableWaitingInfoDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableWaitingInfoDto {
    pub table_id: Uuid,
    pub table_number: String,
    pub static_address_code: String,
    pub store_name: String,
    pub livebrew_url: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct WebRtcSignalPayload {
    pub sdp: Option<String>,
    pub candidate: Option<String>,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u32>,
}
