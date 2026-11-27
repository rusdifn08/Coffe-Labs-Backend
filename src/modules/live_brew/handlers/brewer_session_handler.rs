use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::live_brew::{
        error::LiveBrewError,
        mirror_dto::{CreateRealMirrorSessionRequest, LiveSessionMirrorSnapshot},
        mirror_protocol::{BrewerMirrorCommand, MirrorDownstreamFrame},
        repositories::{branding_seat_repo, real_mirror_session_repo},
        services::{
            mirror_state_machine::{apply_mirror_command, row_to_mirror_snapshot},
            pos_order_sync_bridge, seat_resolution_service,
        },
    },
};

pub async fn create_real_mirror_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<CreateRealMirrorSessionRequest>,
) -> AppResult<(StatusCode, Json<ApiResponse<LiveSessionMirrorSnapshot>>)> {
    let branding = branding_seat_repo::fetch_branding_by_owner(&state.pool, claims.sub).await?;
    let is_business = branding.is_some()
        || claims.tier.eq_ignore_ascii_case("business")
        || claims.role.eq_ignore_ascii_case("admin");
    let store_id = branding.as_ref().map(|b| b.store_id);

    let seat_slug_pair = match (&branding, req.seat_id) {
        (Some(b), Some(sid)) => branding_seat_repo::fetch_seat_by_id(&state.pool, sid)
            .await?
            .map(|s| (b.cafe_slug.clone(), s.seat_slug)),
        _ => None,
    };

    let row = real_mirror_session_repo::create_real_session(
        &state.pool,
        claims.sub,
        &claims.username,
        &req,
        state.config.jwt_secret_str(),
        is_business,
        store_id,
    )
    .await?;

    let branded_ref = seat_slug_pair
        .as_ref()
        .map(|(c, s)| (c.as_str(), s.as_str()));
    let snap = row_to_mirror_snapshot(&row, branded_ref, 0);
    state.live_brew_hub.mirror_hub.publish_snapshot(snap.clone());

    Ok(ApiResponse::created(
        "Sesi Live Brew Real-Time 1:1 aktif & siap dibagikan.",
        snap,
    ))
}

pub async fn dispatch_brewer_command_rest(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
    Json(cmd): Json<BrewerMirrorCommand>,
) -> AppResult<Json<ApiResponse<LiveSessionMirrorSnapshot>>> {
    let updated = apply_and_broadcast_command(&state, claims.sub, &claims.role, session_id, cmd)
        .await?;
    Ok(ApiResponse::ok("State Live Brew 1:1 diperbarui.", updated))
}

pub async fn end_real_mirror_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<bool>>> {
    let row = real_mirror_session_repo::fetch_by_id(&state.pool, session_id)
        .await?
        .ok_or_else(|| LiveBrewError::SessionNotFound(session_id.to_string()))?;

    if row.brewer_user_id != claims.sub && !claims.role.eq_ignore_ascii_case("admin") {
        return Err(LiveBrewError::UnauthorizedBarista.into());
    }

    real_mirror_session_repo::archive_session(&state.pool, session_id).await?;
    state.live_brew_hub.mirror_hub.remove_cached_session(session_id);

    if let (Some(store_id), Some(seat_id)) = (row.store_id, row.seat_id) {
        if let (Ok(Some(branding)), Ok(Some(seat))) = (
            branding_seat_repo::fetch_branding_by_store(&state.pool, store_id).await,
            branding_seat_repo::fetch_seat_by_id(&state.pool, seat_id).await,
        ) {
            if let Ok(res) = seat_resolution_service::build_seat_resolution(
                &state.pool,
                &state.live_brew_hub.mirror_hub,
                &branding,
                &seat,
            )
            .await
            {
                state
                    .live_brew_hub
                    .mirror_hub
                    .publish_seat_update(seat_id, res);
            }
        }
    }

    Ok(ApiResponse::ok(
        "Sesi siaran Live Brew berhasil diakhiri.",
        true,
    ))
}

pub async fn brewer_mirror_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = real_mirror_session_repo::fetch_by_id(&state.pool, session_id)
        .await?
        .ok_or_else(|| AppError::from(LiveBrewError::SessionNotFound(session_id.to_string())))?;

    if row.brewer_user_id != claims.sub && !claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::from(LiveBrewError::UnauthorizedBarista));
    }

    Ok(ws.on_upgrade(move |socket| {
        handle_brewer_mirror_ws(socket, state, claims.sub, claims.role, session_id)
    }))
}

async fn handle_brewer_mirror_ws(
    mut socket: WebSocket,
    state: AppState,
    brewer_id: Uuid,
    role: String,
    session_id: Uuid,
) {
    let mut rx = state.live_brew_hub.mirror_hub.subscribe_session(session_id);
    loop {
        tokio::select! {
            inbound = socket.recv() => {
                let Some(Ok(msg)) = inbound else { break };
                match msg {
                    Message::Text(txt) => {
                        if let Ok(cmd) = serde_json::from_str::<BrewerMirrorCommand>(&txt) {
                            if let BrewerMirrorCommand::NtpPing { client_send_ms } = cmd {
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
                                continue;
                            }
                            let _ = apply_and_broadcast_command(&state, brewer_id, &role, session_id, cmd).await;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            out = rx.recv() => {
                if let Ok(frame) = out {
                    if let Ok(json) = serde_json::to_string(frame.as_ref()) {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn apply_and_broadcast_command(
    state: &AppState,
    user_id: Uuid,
    role: &str,
    session_id: Uuid,
    cmd: BrewerMirrorCommand,
) -> Result<LiveSessionMirrorSnapshot, LiveBrewError> {
    let row = real_mirror_session_repo::fetch_by_id(&state.pool, session_id)
        .await?
        .ok_or_else(|| LiveBrewError::SessionNotFound(session_id.to_string()))?;

    if row.brewer_user_id != user_id && !role.eq_ignore_ascii_case("admin") {
        return Err(LiveBrewError::UnauthorizedBarista);
    }

    let hub = &state.live_brew_hub.mirror_hub;
    let mut snap = hub
        .get_cached_snapshot(session_id)
        .unwrap_or_else(|| row_to_mirror_snapshot(&row, None, hub.active_viewers(session_id)));

    let now_ms = Utc::now().timestamp_millis();
    let is_complete = matches!(cmd, BrewerMirrorCommand::CompleteAndServe { .. });
    if apply_mirror_command(&mut snap, &cmd, now_ms) {
        real_mirror_session_repo::persist_snapshot_state(&state.pool, &snap).await?;
        hub.publish_snapshot(snap.clone());
        if is_complete {
            pos_order_sync_bridge::sync_pos_and_inventory_on_complete(&state.pool, &snap).await;
        }
    }
    Ok(snap)
}
