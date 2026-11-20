use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::ValidatedJson,
        AppState,
    },
    modules::live_brew::{
        dto::SendCheerRequest,
        error::LiveBrewError,
        mirror_dto::NtpCalibrationDto,
        repositories::real_mirror_session_repo,
        services::seat_resolution_service,
    },
};

#[derive(Debug, Clone, Deserialize)]
pub struct NtpQuery {
    pub client_send_ms: Option<i64>,
}

/// Endpoint kalibrasi Cristian's Clock Offset (< 25ms cross-device drift).
pub async fn calibrate_ntp_clock(
    Query(q): Query<NtpQuery>,
) -> AppResult<Json<ApiResponse<NtpCalibrationDto>>> {
    let recv_ms = Utc::now().timestamp_millis();
    let dto = NtpCalibrationDto {
        client_send_ms: q.client_send_ms.unwrap_or(recv_ms),
        server_recv_ms: recv_ms,
        server_send_ms: Utc::now().timestamp_millis(),
    };
    Ok(ApiResponse::ok("Kalibrasi NTP server berhasil.", dto))
}

pub async fn send_mirror_cheer(
    State(state): State<AppState>,
    Path(target): Path<String>,
    ValidatedJson(req): ValidatedJson<SendCheerRequest>,
) -> AppResult<Json<ApiResponse<i32>>> {
    if !state
        .live_brew_hub
        .check_cheer_rate_limit(&format!("v2:{target}"))
    {
        return Err(LiveBrewError::CheerRateLimited.into());
    }
    let resolved = seat_resolution_service::resolve_by_share_code(
        &state.pool,
        &state.live_brew_hub.mirror_hub,
        &target,
    )
    .await?;
    let Some(snap) = resolved.active_brew_session else {
        return Err(LiveBrewError::SessionNotFound(target).into());
    };
    let total = real_mirror_session_repo::increment_cheers(&state.pool, snap.session_id).await?;
    state
        .live_brew_hub
        .mirror_hub
        .publish_cheer(snap.session_id, req.cheer_kind, total);
    Ok(ApiResponse::ok("Apresiasi dikirim ke Barista!", total))
}
