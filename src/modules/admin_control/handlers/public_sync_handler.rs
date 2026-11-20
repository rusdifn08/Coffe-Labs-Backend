use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::{
    core::{error::ApiResponse, AppState},
    modules::admin_control::dto::ControlWsFrame,
};

const ADMIN_CONTROL_PANEL_HTML: &str = include_str!("../../../../admin-panel/index.html");

#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    pub since_version: Option<i64>,
}

/// Menyajikan Backend Web Control Panel (Admin Login JWT, Lock/Unlock Kartu & Harga Pro/Business).
pub async fn serve_web_control_panel() -> Html<&'static str> {
    Html(ADMIN_CONTROL_PANEL_HTML)
}

/// Endpoint publik zero-DB-hit (~15ns dari `ArcSwap`) dengan dukungan `304 Not Modified`.
pub async fn get_control_snapshot(
    State(state): State<AppState>,
    Query(q): Query<SnapshotQuery>,
) -> Response {
    let snap = state.admin_control.load_snapshot_with_clients();
    if let Some(since) = q.since_version {
        if since >= snap.config_version && since > 0 {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }
    ApiResponse::ok("Snapshot konfigurasi real-time Coffee Labs diperoleh.", snap).into_response()
}

/// Global WebSocket Hub untuk push real-time perubahan lock kartu & harga ke semua perangkat (< 50ms).
pub async fn control_ws_upgrade(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_control_socket(state, socket))
}

async fn handle_control_socket(state: AppState, socket: WebSocket) {
    let (mut rx, _) = state.admin_control.register_ws_client();
    let (mut sender, mut receiver) = socket.split();

    let initial_snap = state.admin_control.load_snapshot_with_clients();
    let init_frame = ControlWsFrame::SnapshotSynced {
        snapshot: initial_snap,
    };
    if let Ok(text) = serde_json::to_string(&init_frame) {
        if sender.send(Message::Text(text.into())).await.is_err() {
            state.admin_control.unregister_ws_client();
            return;
        }
    }

    let mut heartbeat = tokio::time::interval(Duration::from_secs(25));
    heartbeat.tick().await;

    loop {
        tokio::select! {
            recv = rx.recv() => {
                match recv {
                    Ok(frame_json) => {
                        if sender.send(Message::Text((*frame_json).clone().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let fresh = state.admin_control.load_snapshot_with_clients();
                        let resync = ControlWsFrame::SnapshotSynced { snapshot: fresh };
                        if let Ok(text) = serde_json::to_string(&resync) {
                            if sender.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = heartbeat.tick() => {
                let cur = state.admin_control.load_snapshot_with_clients();
                let hb = ControlWsFrame::Heartbeat {
                    config_version: cur.config_version,
                    connected_clients: cur.connected_clients,
                };
                if let Ok(text) = serde_json::to_string(&hb) {
                    if sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(Message::Ping(payload))) => {
                        let _ = sender.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    state.admin_control.unregister_ws_client();
}
