use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::{error::NotifError, models::NotificationInboxRow};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterDeviceTokenReq {
    #[validate(length(min = 3, max = 128))]
    pub device_installation_id: String,
    #[validate(length(min = 3, max = 16))]
    pub platform: String,
    #[validate(length(min = 8, max = 4096))]
    pub fcm_token: String,
    pub apns_voip_or_live_activity_token: Option<String>,
    pub app_version: Option<String>,
    pub device_model: Option<String>,
    pub is_kiosk_aod_active: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateKioskModeReq {
    #[validate(length(min = 3, max = 128))]
    pub device_installation_id: String,
    pub is_kiosk_aod_active: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UnregisterDeviceReq {
    #[validate(length(min = 3, max = 128))]
    pub device_installation_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ActiveChatRoomReq {
    pub conversation_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct InboxQuery {
    pub category: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct InboxPageResp {
    pub items: Vec<NotificationInboxRow>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct UpdatePreferencesReq {
    pub enable_chat_messages: bool,
    pub enable_follow_follback: bool,
    pub enable_thread_replies: bool,
    pub enable_likes_and_recipe_tries: bool,
    pub enable_live_brew_and_streak: bool,
    pub enable_transactional_billing: bool,
    #[validate(range(min = 0, max = 1439))]
    pub quiet_hours_start_minute: Option<i32>,
    #[validate(range(min = 0, max = 1439))]
    pub quiet_hours_end_minute: Option<i32>,
    #[validate(length(min = 2, max = 64))]
    pub timezone: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationCursor {
    pub updated_at: DateTime<Utc>,
    pub id: Uuid,
}

impl NotificationCursor {
    pub fn encode(&self) -> String {
        let raw = format!("{}|{}", self.updated_at.timestamp_micros(), self.id);
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    pub fn decode(token: &str) -> Result<Self, NotifError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(token.trim())
            .map_err(|_| NotifError::InvalidPayload("Cursor notifikasi tidak valid.".to_string()))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| NotifError::InvalidPayload("Format cursor UTF-8 tidak valid.".to_string()))?;
        let (ts_raw, id_raw) = text
            .split_once('|')
            .ok_or_else(|| NotifError::InvalidPayload("Struktur cursor tidak lengkap.".to_string()))?;
        let micros: i64 = ts_raw
            .parse()
            .map_err(|_| NotifError::InvalidPayload("Timestamp cursor tidak valid.".to_string()))?;
        let updated_at = DateTime::from_timestamp_micros(micros)
            .ok_or_else(|| NotifError::InvalidPayload("Timestamp cursor di luar rentang.".to_string()))?;
        let id = Uuid::parse_str(id_raw)
            .map_err(|_| NotifError::InvalidPayload("UUID cursor tidak valid.".to_string()))?;
        Ok(Self { updated_at, id })
    }
}
