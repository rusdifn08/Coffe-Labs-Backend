use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct BusinessBrandingRow {
    pub store_id: Uuid,
    #[allow(dead_code)]
    pub owner_user_id: Uuid,
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

#[derive(Debug, Clone, FromRow)]
pub struct CafeSeatRow {
    pub id: Uuid,
    pub store_id: Uuid,
    pub seat_kind: String,
    pub seat_label: String,
    pub seat_slug: String,
    pub zone_area: String,
    pub capacity_persons: i16,
    pub numeric_fallback_code: String,
    pub active_live_session_id: Option<Uuid>,
    pub active_pos_order_id: Option<Uuid>,
    pub active_pos_order_number: Option<String>,
    pub active_pos_order_status: Option<String>,
    pub active_pos_items_summary: Option<String>,
    pub active_pos_customer_name: Option<String>,
    pub qr_scan_count: i64,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct LiveBrewRealSessionRow {
    pub id: Uuid,
    pub share_code: String,
    pub is_business_session: bool,
    pub store_id: Option<Uuid>,
    pub seat_id: Option<Uuid>,
    pub pos_order_id: Option<Uuid>,
    pub pos_order_number: Option<String>,
    pub pos_order_status: Option<String>,
    pub brewer_user_id: Uuid,
    pub brewer_display_name: String,
    pub brewer_avatar_url: Option<String>,
    pub customer_name: String,
    pub phase: String,
    pub state_version: i64,
    pub selected_method_id: Option<String>,
    pub selected_method_name: Option<String>,
    pub selected_recipe_id: Option<Uuid>,
    pub selected_recipe_title: Option<String>,
    pub recipe_snapshot_json: Value,
    pub selected_bean_id: Option<Uuid>,
    pub bean_snapshot_json: Value,
    pub timer_status: String,
    pub timer_accumulated_ms: i64,
    pub timer_anchor_server_ms: Option<i64>,
    pub current_step_index: i16,
    pub current_weight_g: f32,
    pub current_flow_rate_ml_s: f32,
    pub current_temp_c: f32,
    pub telemetry_curve_json: Value,
    pub cheers_count: i32,
    pub active_viewers_count: i32,
}

/// Normalisasi slug kafe atau meja menjadi huruf kecil + tanda hubung aman untuk URL.
pub fn slugify_livebrew_segment(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in raw.trim().chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "coffee-labs".to_string()
    } else {
        trimmed
    }
}

pub fn build_branded_seat_url(cafe_slug: &str, seat_slug: &str) -> String {
    format!("https://coffeelabs.com/livebrew/{cafe_slug}/{seat_slug}")
}

pub fn build_branded_seat_deeplink(cafe_slug: &str, seat_slug: &str) -> String {
    format!("coffeelabs://livebrew/{cafe_slug}/{seat_slug}")
}

pub fn build_personal_share_url(share_code: &str) -> String {
    format!("https://coffeelabs.com/livebrew/{share_code}")
}
