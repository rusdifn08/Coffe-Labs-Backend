use axum::{extract::State, Json};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppError, AppResult},
    extractors::{RequireTier, TierBusiness, ValidatedJson},
    AppState,
};
use crate::modules::business_pos::{
    kiosk_dto::{EmergencyUnlockDto, EmergencyUnlockResponse, KioskHeartbeatDto, KioskHeartbeatResponse},
    services::BusinessPosService,
};

fn parse_user_id(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::Unauthorized("ID Pengguna pada token tidak valid.".to_string()))
}

/// Menghasilkan 6-digit dynamic recovery PIN yang berubah setiap 15 menit
fn generate_dynamic_recovery_pin(store_id: &Uuid) -> String {
    let time_window = Utc::now().timestamp() / 900; // 15 menit
    let seed = format!("{}:{}:kiosk_emergency_recovery_salt", store_id, time_window);
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let hash = hasher.finalize();

    // Ambil 4 byte pertama untuk angka 6-digit (100000 - 999999)
    let num = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]);
    format!("{:06}", 100_000 + (num % 900_000))
}

pub async fn kiosk_heartbeat(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<KioskHeartbeatDto>,
) -> AppResult<Json<ApiResponse<KioskHeartbeatResponse>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let _config = BusinessPosService::get_config(&state.pool, user_id).await?;

    tracing::info!(
        device_id = %dto.device_id,
        is_locked = dto.is_locked,
        mode = ?dto.kiosk_mode,
        battery = ?dto.battery_level,
        "Kiosk telemetry heartbeat received"
    );

    Ok(ApiResponse::ok(
        "Telemetry heartbeat diterima.",
        KioskHeartbeatResponse {
            acknowledged: true,
            remote_unlock_requested: false,
            server_time_utc: Utc::now().timestamp(),
        },
    ))
}

pub async fn emergency_unlock(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(_dto): ValidatedJson<EmergencyUnlockDto>,
) -> AppResult<Json<ApiResponse<EmergencyUnlockResponse>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let config = BusinessPosService::get_config(&state.pool, user_id).await?;
    let recovery_pin = generate_dynamic_recovery_pin(&config.id);

    tracing::warn!(
        store_id = %config.id,
        user_id = %user_id,
        "Emergency Kiosk Unlock PIN requested"
    );

    Ok(ApiResponse::ok(
        "Kunci pemulihan darurat Kiosk berhasil dibuat. Berlaku selama 15 menit.",
        EmergencyUnlockResponse {
            unlock_issued: true,
            master_recovery_pin: recovery_pin,
            expires_in_seconds: 900,
        },
    ))
}

pub async fn get_recovery_pin(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<String>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let config = BusinessPosService::get_config(&state.pool, user_id).await?;
    let recovery_pin = generate_dynamic_recovery_pin(&config.id);

    Ok(ApiResponse::ok(
        "Master Recovery PIN darurat berhasil diambil.",
        recovery_pin,
    ))
}
