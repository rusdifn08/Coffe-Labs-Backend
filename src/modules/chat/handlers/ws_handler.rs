use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header, HeaderMap},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tracing::debug;
use uuid::Uuid;

use crate::{
    core::{error::AppError, security::Claims, AppState},
    modules::chat::{
        dto::WsConnectQuery,
        handlers::ws_dispatcher::{dispatch_client_frame, send_frame_error},
        protocol::{ClientFrame, ServerFrame},
        services::{
            moderation_guard::SocketRateLimiter, presence_service::broadcast_presence_change,
        },
    },
};

const MAX_WS_FRAME_BYTES: usize = 16 * 1024;
const MAX_WS_MSG_BYTES: usize = 32 * 1024;

pub async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsConnectQuery>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let raw_token = extract_bearer_or_query(&headers, query.token.as_deref()).ok_or_else(|| {
        AppError::Unauthorized("Token JWT diperlukan untuk koneksi chat real-time.".to_string())
    })?;
    let claims = state.jwt_keys.verify_token(raw_token)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("User ID pada token tidak valid.".to_string()))?;
    let use_msgpack = query.format.as_deref() == Some("msgpack");

    Ok(ws
        .max_frame_size(MAX_WS_FRAME_BYTES)
        .max_message_size(MAX_WS_MSG_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, state, user_id, claims, use_msgpack)))
}

fn extract_bearer_or_query<'a>(
    headers: &'a HeaderMap,
    query_token: Option<&'a str>,
) -> Option<&'a str> {
    if let Some(auth_val) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(stripped) = auth_val.strip_prefix("Bearer ") {
            let trimmed = stripped.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    query_token.map(str::trim).filter(|s| !s.is_empty())
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    user_id: Uuid,
    claims: Claims,
    use_msgpack: bool,
) {
    let (session_id, mut rx, became_online) = state.chat_hub.register(user_id);
    state.chat_presence.mark_online(user_id, &state.chat_bridge);

    if became_online {
        let pool = state.pool.clone();
        let bridge = state.chat_bridge.clone();
        tokio::spawn(async move {
            broadcast_presence_change(&pool, &bridge, user_id, true, None).await;
        });
    }

    let (mut ws_sink, mut ws_stream) = socket.split();
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            let Some(msg) = encode_server_frame(frame.as_ref(), use_msgpack) else {
                continue;
            };
            if ws_sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    let limiter = SocketRateLimiter::new();
    while let Some(next_item) = ws_stream.next().await {
        let Ok(msg) = next_item else { break };
        match msg {
            Message::Text(text) => match serde_json::from_str::<ClientFrame>(&text) {
                Ok(frame) => {
                    dispatch_client_frame(&state, &limiter, user_id, &claims.username, frame).await;
                }
                Err(err) => send_frame_error(&state, user_id, None, "INVALID_JSON", &err.to_string()),
            },
            Message::Binary(bytes) => match rmp_serde::from_slice::<ClientFrame>(&bytes) {
                Ok(frame) => {
                    dispatch_client_frame(&state, &limiter, user_id, &claims.username, frame).await;
                }
                Err(err) => {
                    send_frame_error(&state, user_id, None, "INVALID_MSGPACK", &err.to_string())
                }
            },
            Message::Ping(_) | Message::Pong(_) => {
                state
                    .chat_presence
                    .refresh_heartbeat(user_id, &state.chat_bridge);
            }
            Message::Close(_) => break,
        }
    }

    writer_task.abort();
    let became_offline = state.chat_hub.unregister(user_id, session_id);
    let last_seen = state.chat_presence.mark_offline(user_id, &state.chat_bridge);
    debug!(user_id = %user_id, session_id = %session_id, "WS chat terputus bersih");

    if became_offline {
        let state_bg = state.clone();
        tokio::spawn(async move {
            crate::modules::notifications::handlers::device_handler::sync_active_room_redis(
                &state_bg, user_id, None,
            )
            .await;
            broadcast_presence_change(
                &state_bg.pool,
                &state_bg.chat_bridge,
                user_id,
                false,
                Some(last_seen),
            )
            .await;
        });
    }
}

fn encode_server_frame(frame: &ServerFrame, use_msgpack: bool) -> Option<Message> {
    if use_msgpack {
        rmp_serde::to_vec_named(frame)
            .ok()
            .map(|b| Message::Binary(b.into()))
    } else {
        serde_json::to_string(frame)
            .ok()
            .map(|s| Message::Text(s.into()))
    }
}
