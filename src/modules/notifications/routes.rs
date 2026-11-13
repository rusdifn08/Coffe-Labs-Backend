use axum::{
    routing::{get, patch, post, put},
    Router,
};

use crate::{
    core::AppState,
    modules::notifications::handlers::{device_handler, inbox_handler},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/notifications/devices/token",
            put(device_handler::put_register_device_token)
                .delete(device_handler::delete_unregister_device_token),
        )
        .route(
            "/api/v1/notifications/devices/kiosk-mode",
            patch(device_handler::patch_kiosk_mode),
        )
        .route(
            "/api/v1/notifications/presence/active-room",
            post(device_handler::post_set_active_chat_room),
        )
        .route("/api/v1/notifications/inbox", get(inbox_handler::get_inbox))
        .route(
            "/api/v1/notifications/unread-count",
            get(inbox_handler::get_unread_count),
        )
        .route(
            "/api/v1/notifications/inbox/read-all",
            post(inbox_handler::post_mark_all_read),
        )
        .route(
            "/api/v1/notifications/inbox/{id}/read",
            post(inbox_handler::post_mark_read),
        )
        .route(
            "/api/v1/notifications/preferences",
            get(inbox_handler::get_preferences).put(inbox_handler::put_preferences),
        )
}
