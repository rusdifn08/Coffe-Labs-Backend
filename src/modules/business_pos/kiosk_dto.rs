use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct KioskHeartbeatDto {
    #[validate(length(min = 1, max = 100, message = "Device ID harus valid (1 - 100 karakter)."))]
    pub device_id: String,
    #[validate(range(min = 0.0, max = 1.0, message = "Battery level antara 0.0 dan 1.0."))]
    pub battery_level: Option<f32>,
    pub is_charging: Option<bool>,
    pub kiosk_mode: Option<String>,
    pub is_locked: bool,
    pub app_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KioskHeartbeatResponse {
    pub acknowledged: bool,
    pub remote_unlock_requested: bool,
    pub server_time_utc: i64,
}

#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct EmergencyUnlockDto {
    pub device_id: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmergencyUnlockResponse {
    pub unlock_issued: bool,
    pub master_recovery_pin: String,
    pub expires_in_seconds: u32,
}
