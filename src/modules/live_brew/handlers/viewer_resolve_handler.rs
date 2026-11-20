use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        AppState,
    },
    modules::live_brew::{
        mirror_dto::BrandedSeatResolutionDto,
        mirror_protocol::{BrewerMirrorCommand, MirrorDownstreamFrame},
        services::seat_resolution_service,
    },
};

pub async fn resolve_by_code(
    State(state): State<AppState>,
    Path(share_code): Path<String>,
) -> AppResult<Json<ApiResponse<BrandedSeatResolutionDto>>> {
    let data = seat_resolution_service::resolve_by_share_code(
        &state.pool,
        &state.live_brew_hub.mirror_hub,
        &share_code,
    )
    .await?;
    Ok(ApiResponse::ok("Sesi Live Brew berhasil ditemukan.", data))
}

pub async fn resolve_by_cafe_seat(
    State(state): State<AppState>,
    Path((cafe_slug, seat_slug)): Path<(String, String)>,
) -> AppResult<Json<ApiResponse<BrandedSeatResolutionDto>>> {
    let data = seat_resolution_service::resolve_by_cafe_and_seat(
        &state.pool,
        &state.live_brew_hub.mirror_hub,
        &cafe_slug,
        &seat_slug,
    )
    .await?;
    Ok(ApiResponse::ok(
        "Status meja/kursi kafe berhasil dimuat.",
        data,
    ))
}

pub async fn resolve_stream_target(
    state: &AppState,
    target: &str,
) -> Result<BrandedSeatResolutionDto, AppError> {
    if let Some((cafe_slug, seat_slug)) = target.split_once("__") {
        return seat_resolution_service::resolve_by_cafe_and_seat(
            &state.pool,
            &state.live_brew_hub.mirror_hub,
            cafe_slug,
            seat_slug,
        )
        .await
        .map_err(AppError::from);
    }
    seat_resolution_service::resolve_by_share_code(
        &state.pool,
        &state.live_brew_hub.mirror_hub,
        target,
    )
    .await
    .map_err(AppError::from)
}

pub async fn viewer_v2_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(target): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let resolved = resolve_stream_target(&state, &target).await?;
    Ok(ws.on_upgrade(move |socket| handle_viewer_v2_ws(socket, state, resolved)))
}

async fn handle_viewer_v2_ws(
    mut socket: WebSocket,
    state: AppState,
    resolved: BrandedSeatResolutionDto,
) {
    let hub = Arc::clone(&state.live_brew_hub.mirror_hub);
    let session_id: Option<Uuid> = resolved.active_brew_session.as_ref().map(|s| s.session_id);
    let seat_id: Option<Uuid> = resolved.seat.as_ref().map(|s| s.id);

    if let Some(sid) = session_id {
        hub.adjust_viewers(sid, 1);
    }

    // Kirim initial state secara instan (< 10ms)
    let init_frame = if let Some(ref snap) = resolved.active_brew_session {
        MirrorDownstreamFrame::MirrorSnapshot(snap.clone())
    } else {
        MirrorDownstreamFrame::SeatPosUpdate(resolved.clone())
    };
    if let Ok(json) = serde_json::to_string(&init_frame) {
        if socket.send(Message::Text(json.into())).await.is_err() {
            if let Some(sid) = session_id {
                hub.adjust_viewers(sid, -1);
            }
            return;
        }
    }

    let mut session_rx = session_id.map(|sid| hub.subscribe_session(sid));
    let mut seat_rx = seat_id.map(|stid| hub.subscribe_seat(stid));

    loop {
        tokio::select! {
            inbound = socket.recv() => {
                let Some(Ok(msg)) = inbound else { break };
                match msg {
                    Message::Text(txt) => {
                        if let Ok(BrewerMirrorCommand::NtpPing { client_send_ms }) = serde_json::from_str(&txt) {
                            let recv_ms = Utc::now().timestamp_millis();
                            let pong = MirrorDownstreamFrame::NtpPong {
                                client_send_ms,
                                server_recv_ms: recv_ms,
                                server_send_ms: Utc::now().timestamp_millis(),
                            };
                            if let Ok(json) = serde_json::to_string(&pong) {
                                if socket.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            ev = async {
                match session_rx.as_mut() {
                    Some(rx) => rx.recv().await.ok(),
                    None => std::future::pending().await,
                }
            } => {
                if let Some(frame) = ev {
                    if let Ok(json) = serde_json::to_string(frame.as_ref()) {
                        if socket.send(Message::Text(json.into())).await.is_err() { break; }
                    }
                }
            }
            ev = async {
                match seat_rx.as_mut() {
                    Some(rx) => rx.recv().await.ok(),
                    None => std::future::pending().await,
                }
            } => {
                if let Some(frame) = ev {
                    if let Ok(json) = serde_json::to_string(frame.as_ref()) {
                        if socket.send(Message::Text(json.into())).await.is_err() { break; }
                    }
                }
            }
        }
    }

    if let Some(sid) = session_id {
        hub.adjust_viewers(sid, -1);
    }
}
