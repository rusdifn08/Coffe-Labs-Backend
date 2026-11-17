use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{error::{ApiResponse, AppError, AppResult}, extractors::{AuthUser, ValidatedJson}, AppState},
    modules::live_brew::{
        dto::WebRtcSignalPayload,
        error::LiveBrewError,
        protocol::ViewerDownstreamEvent,
        repositories::live_session_repo,
    },
};

pub async fn whip_publish(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
    body: String,
) -> AppResult<impl IntoResponse> {
    let row = live_session_repo::fetch_by_id(&state.pool, session_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::from(LiveBrewError::SessionNotFound(session_id.to_string())))?;

    if row.barista_user_id != claims.sub && !claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::from(LiveBrewError::UnauthorizedBarista));
    }

    let whep_url = format!("/api/v1/livebrew/rtc/{session_id}/whep");
    state.live_brew_hub.store_whip_offer(session_id, body);
    live_session_repo::set_video_stream_flag(&state.pool, session_id, true, Some(&whep_url))
        .await
        .map_err(AppError::from)?;

    state.live_brew_hub.broadcast_to_session(
        session_id,
        Arc::new(ViewerDownstreamEvent::VideoStatusChanged {
            is_active: true,
            whep_url: Some(whep_url.clone()),
        }),
    );

    Ok((
        StatusCode::CREATED,
        [(header::LOCATION, whep_url.clone())],
        ApiResponse::created("WHIP camera stream aktif.", whep_url),
    ))
}

pub async fn whip_teardown(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let row = live_session_repo::fetch_by_id(&state.pool, session_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::from(LiveBrewError::SessionNotFound(session_id.to_string())))?;

    if row.barista_user_id != claims.sub && !claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::from(LiveBrewError::UnauthorizedBarista));
    }

    state.live_brew_hub.clear_whip(session_id);
    live_session_repo::set_video_stream_flag(&state.pool, session_id, false, None)
        .await
        .map_err(AppError::from)?;

    state.live_brew_hub.broadcast_to_session(
        session_id,
        Arc::new(ViewerDownstreamEvent::VideoStatusChanged {
            is_active: false,
            whep_url: None,
        }),
    );

    Ok(ApiResponse::ok("WHIP camera stream dimatikan.", session_id))
}

pub async fn whep_subscribe(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let sdp = state.live_brew_hub.get_whip_offer(session_id);
    let ice = state.live_brew_hub.take_ice_candidates(session_id);
    Ok(ApiResponse::ok(
        "Informasi sinyal WHEP tersedia.",
        serde_json::json!({
            "session_id": session_id,
            "has_sdp_offer": sdp.is_some(),
            "sdp": sdp,
            "ice_candidates": ice,
        }),
    ))
}

pub async fn patch_ice_candidate(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<WebRtcSignalPayload>,
) -> AppResult<impl IntoResponse> {
    if payload.candidate.is_some() {
        state.live_brew_hub.push_ice_candidate(session_id, &payload);
    }
    Ok(ApiResponse::ok("ICE candidate tersimpan.", session_id))
}
