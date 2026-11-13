use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    core::AppState,
    modules::chat::handlers::{reply_handler, room_handler, sync_handler, ws_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/chat/ws", get(ws_handler::ws_upgrade_handler))
        .route(
            "/api/v1/chat/conversations",
            get(room_handler::list_conversations),
        )
        .route(
            "/api/v1/chat/conversations/direct",
            post(room_handler::create_direct_room),
        )
        .route(
            "/api/v1/chat/conversations/group",
            post(room_handler::create_group_room),
        )
        .route(
            "/api/v1/chat/conversations/{id}/messages",
            get(sync_handler::get_conversation_history).post(reply_handler::post_send_message),
        )
        .route(
            "/api/v1/chat/rooms/{id}/messages",
            post(reply_handler::post_send_message),
        )
        .route("/api/v1/chat/sync-delta", post(sync_handler::sync_delta_http))
        .route("/api/v1/chat/peers", get(room_handler::list_peers))
}
