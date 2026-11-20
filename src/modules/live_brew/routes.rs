use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    core::AppState,
    modules::live_brew::handlers::{
        barista_ws_handler, brewer_session_handler, business_seat_handler, ntp_and_cheer_handler,
        session_table_handler, viewer_resolve_handler, viewer_sse_handler, viewer_stream_handler,
        webrtc_signaling_handler,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        // === LIVE BREW v2.0 REAL-TIME 1:1 MIRROR, NTP & BUSINESS SEAT STUDIO ===
        .route("/api/v1/livebrew/ntp", get(ntp_and_cheer_handler::calibrate_ntp_clock))
        .route(
            "/api/v1/livebrew/v2/sessions",
            post(brewer_session_handler::create_real_mirror_session),
        )
        .route(
            "/api/v1/livebrew/v2/sessions/{session_id}/command",
            post(brewer_session_handler::dispatch_brewer_command_rest),
        )
        .route(
            "/api/v1/livebrew/v2/brewer/ws/{session_id}",
            get(brewer_session_handler::brewer_mirror_ws_upgrade),
        )
        .route(
            "/api/v1/livebrew/resolve/code/{share_code}",
            get(viewer_resolve_handler::resolve_by_code),
        )
        .route(
            "/api/v1/livebrew/resolve/cafe/{cafe_slug}/{seat_slug}",
            get(viewer_resolve_handler::resolve_by_cafe_seat),
        )
        .route(
            "/api/v1/livebrew/v2/stream/{target}/ws",
            get(viewer_resolve_handler::viewer_v2_ws_upgrade),
        )
        .route(
            "/api/v1/livebrew/v2/stream/{target}/sse",
            get(viewer_sse_handler::viewer_v2_sse_stream),
        )
        .route(
            "/api/v1/livebrew/v2/{target}/cheer",
            post(ntp_and_cheer_handler::send_mirror_cheer),
        )
        .route(
            "/api/v1/livebrew/business/branding",
            get(business_seat_handler::get_my_branding)
                .put(business_seat_handler::upsert_my_branding),
        )
        .route(
            "/api/v1/livebrew/business/seats",
            get(business_seat_handler::list_my_seats).post(business_seat_handler::create_seat),
        )
        .route(
            "/api/v1/livebrew/business/seats/batch",
            post(business_seat_handler::create_seat_batch),
        )
        .route(
            "/api/v1/livebrew/business/pos-bind",
            post(business_seat_handler::bind_pos_order_to_seat),
        )
        // === BACKWARDS-COMPATIBLE v1 ENDPOINTS ===
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
