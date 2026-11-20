use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use validator::Validate;

use crate::modules::live_brew::mirror_protocol::{
    CafeSeatKind, LiveBrewMirrorPhase, LiveTimerStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessBrandingDto {
    pub store_id: Uuid,
    pub cafe_slug: String,
    pub cafe_display_name: String,
    pub logo_cloudinary_url: Option<String>,
    pub primary_color_hex: String,
    pub accent_color_hex: String,
    pub welcome_message: String,
    pub wifi_ssid: Option<String>,
    pub wifi_password: Option<String>,
    pub instagram_handle: Option<String>,
    pub show_pos_queue_position: bool,
    pub show_live_bean_details: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpsertBrandingRequest {
    #[validate(length(min = 2, max = 64))]
    pub cafe_slug: String,
    #[validate(length(min = 2, max = 120))]
    pub cafe_display_name: String,
    pub logo_cloudinary_url: Option<String>,
    pub logo_cloudinary_public_id: Option<String>,
    pub primary_color_hex: Option<String>,
    pub accent_color_hex: Option<String>,
    pub welcome_message: Option<String>,
    pub wifi_ssid: Option<String>,
    pub wifi_password: Option<String>,
    pub instagram_handle: Option<String>,
    pub show_pos_queue_position: Option<bool>,
    pub show_live_bean_details: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CafeSeatDto {
    pub id: Uuid,
    pub store_id: Uuid,
    pub seat_kind: CafeSeatKind,
    pub seat_label: String,
    pub seat_slug: String,
    pub zone_area: String,
    pub capacity_persons: i16,
    pub numeric_fallback_code: String,
    pub branded_url: String,
    pub deep_link_url: String,
    pub active_live_session_id: Option<Uuid>,
    pub active_pos_order_id: Option<Uuid>,
    pub active_pos_order_number: Option<String>,
    pub active_pos_order_status: Option<String>,
    pub qr_scan_count: i64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSeatRequest {
    pub seat_kind: Option<CafeSeatKind>,
    #[validate(length(min = 1, max = 40))]
    pub seat_label: String,
    pub seat_slug: Option<String>,
    pub zone_area: Option<String>,
    pub capacity_persons: Option<i16>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSeatBatchRequest {
    pub seat_kind: Option<CafeSeatKind>,
    #[validate(length(min = 1, max = 24))]
    pub prefix: String,
    pub start_number: u16,
    pub end_number: u16,
    pub zone_area: Option<String>,
    pub capacity_persons: Option<i16>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct PosOrderBindRequest {
    pub seat_id: Uuid,
    pub pos_order_id: Option<Uuid>,
    #[validate(length(min = 1, max = 32))]
    pub pos_order_number: String,
    pub pos_order_status: Option<String>,
    pub customer_name: Option<String>,
    pub items_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PosOrderProgressDto {
    pub pos_order_id: Option<Uuid>,
    pub pos_order_number: String,
    pub pos_order_status: String,
    pub queue_position_ahead: i64,
    pub items_summary: String,
    pub customer_name: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateRealMirrorSessionRequest {
    pub seat_id: Option<Uuid>,
    pub pos_order_id: Option<Uuid>,
    pub pos_order_number: Option<String>,
    pub customer_name: Option<String>,
    pub selected_method_id: Option<String>,
    pub selected_method_name: Option<String>,
    pub selected_recipe_id: Option<Uuid>,
    pub selected_recipe_title: Option<String>,
    pub recipe_snapshot_json: Option<Value>,
    pub selected_bean_id: Option<Uuid>,
    pub bean_snapshot_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSessionMirrorSnapshot {
    pub session_id: Uuid,
    pub share_code: String,
    pub share_url: String,
    pub deep_link_url: String,
    pub is_business_session: bool,
    pub store_id: Option<Uuid>,
    pub seat_id: Option<Uuid>,
    pub pos_order_id: Option<Uuid>,
    pub pos_order_number: Option<String>,
    pub pos_order_status: Option<String>,
    pub brewer_display_name: String,
    pub brewer_avatar_url: Option<String>,
    pub customer_name: String,
    pub phase: LiveBrewMirrorPhase,
    pub phase_label_id: String,
    pub state_version: i64,
    pub selected_method_id: Option<String>,
    pub selected_method_name: Option<String>,
    pub selected_recipe_id: Option<Uuid>,
    pub selected_recipe_title: Option<String>,
    pub recipe_snapshot_json: Value,
    pub selected_bean_id: Option<Uuid>,
    pub bean_snapshot_json: Value,
    pub timer_status: LiveTimerStatus,
    pub timer_accumulated_ms: i64,
    pub timer_anchor_server_ms: Option<i64>,
    pub current_step_index: i16,
    pub current_weight_g: f32,
    pub current_flow_rate_ml_s: f32,
    pub current_temp_c: f32,
    pub telemetry_curve_json: Value,
    pub cheers_count: i32,
    pub active_viewers_count: i32,
    pub server_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandedSeatResolutionDto {
    pub resolution_kind: String,
    pub branding: Option<BusinessBrandingDto>,
    pub seat: Option<CafeSeatDto>,
    pub pos_order_progress: Option<PosOrderProgressDto>,
    pub active_brew_session: Option<LiveSessionMirrorSnapshot>,
    pub server_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtpCalibrationDto {
    pub client_send_ms: i64,
    pub server_recv_ms: i64,
    pub server_send_ms: i64,
}
