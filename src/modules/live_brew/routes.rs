use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    core::AppState,
    modules::live_brew::handlers::{
        barista_ws_handler, session_table_handler, viewer_stream_handler, webrtc_signaling_handler,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/livebrew/sessions", post(session_table_handler::create_live_session))
        .route(
            "/api/v1/livebrew/tables",
            get(session_table_handler::list_tables).post(session_table_handler::create_table),
        )
        .route(
            "/api/v1/livebrew/tables/{table_id}/bind",
            post(session_table_handler::bind_table_session),
        )
        .route(
            "/api/v1/livebrew/resolve/{address_code}",
            get(session_table_handler::resolve_live_address),
        )
        .route(
            "/api/v1/livebrew/barista/ws/{session_id}",
            get(barista_ws_handler::barista_ws_upgrade),
        )
        .route(
            "/api/v1/livebrew/stream/{address_code}/sse",
            get(viewer_stream_handler::viewer_sse_stream),
        )
        .route(
            "/api/v1/livebrew/stream/{address_code}/ws",
            get(viewer_stream_handler::viewer_ws_upgrade),
        )
        .route(
            "/api/v1/livebrew/{address_code}/cheer",
            post(viewer_stream_handler::send_cheer_reaction),
        )
        .route(
            "/api/v1/livebrew/rtc/{session_id}/whip",
            post(webrtc_signaling_handler::whip_publish)
                .delete(webrtc_signaling_handler::whip_teardown),
        )
        .route(
            "/api/v1/livebrew/rtc/{session_id}/whep",
            post(webrtc_signaling_handler::whep_subscribe)
                .get(webrtc_signaling_handler::whep_subscribe),
        )
        .route(
            "/api/v1/livebrew/rtc/{session_id}/ice",
            patch(webrtc_signaling_handler::patch_ice_candidate),
        )
}
