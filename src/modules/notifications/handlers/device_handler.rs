use axum::{extract::State, response::IntoResponse, Json};
use fred::{interfaces::KeysInterface, types::Expiration};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::notifications::{
        builders::build_chat_message_event,
        dto::{ActiveChatRoomReq, RegisterDeviceTokenReq, UnregisterDeviceReq, UpdateKioskModeReq},
        events::ActorSummary,
        repositories::device_token_repo,
    },
};

pub async fn put_register_device_token(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterDeviceTokenReq>,
) -> AppResult<impl IntoResponse> {
    let row = device_token_repo::upsert_device_token(&state.pool, claims.sub, &req)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Token perangkat push berhasil didaftarkan.", row))
}

pub async fn patch_kiosk_mode(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<UpdateKioskModeReq>,
) -> AppResult<impl IntoResponse> {
    device_token_repo::update_kiosk_aod_flag(
        &state.pool,
        claims.sub,
        &req.device_installation_id,
        req.is_kiosk_aod_active,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Status lock Kiosk/AOD perangkat berhasil diperbarui.",
        json!({ "is_kiosk_aod_active": req.is_kiosk_aod_active }),
    ))
}

pub async fn delete_unregister_device_token(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<UnregisterDeviceReq>,
) -> AppResult<impl IntoResponse> {
    device_token_repo::deactivate_by_installation(&state.pool, claims.sub, &req.device_installation_id)
        .await
        .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Token perangkat push berhasil dinonaktifkan.",
        json!({ "unregistered": true }),
    ))
}

pub async fn sync_active_room_redis(
    state: &AppState,
    user_id: Uuid,
    conversation_id: Option<Uuid>,
) {
    state.chat_presence.set_active_room(user_id, conversation_id);
    let Some(pool) = state.chat_bridge.redis_pool().cloned() else {
        return;
    };
    let key = format!("chat:active_room:{user_id}");
    match conversation_id {
        Some(room_id) => {
            let _ = pool
                .next()
                .set::<(), _, _>(key, room_id.to_string(), Some(Expiration::EX(3600)), None, false)
                .await;
        }
        None => {
            let _ = pool.next().del::<i64, _>(key).await;
        }
    }
}

pub async fn post_set_active_chat_room(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<ActiveChatRoomReq>,
) -> AppResult<impl IntoResponse> {
    sync_active_room_redis(&state, claims.sub, req.conversation_id).await;
    Ok(ApiResponse::ok(
        "Presensi ruang obrolan aktif diperbarui.",
        json!({ "conversation_id": req.conversation_id }),
    ))
}

pub async fn get_push_device_status(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let tokens = device_token_repo::list_active_tokens(&state.pool, claims.sub)
        .await
        .map_err(AppError::from)?;
    let platforms: Vec<String> = tokens.iter().map(|t| t.platform.clone()).collect();
    Ok(ApiResponse::ok(
        "Status perangkat push notification dimuat.",
        json!({
            "fcm_server_configured": state.fcm_oauth.is_configured(),
            "active_device_tokens": tokens.len(),
            "platforms": platforms,
            "ws_connected": state.chat_hub.is_local_online(claims.sub),
        }),
    ))
}

pub async fn post_test_push_notification(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let tokens = device_token_repo::list_active_tokens(&state.pool, claims.sub)
        .await
        .map_err(AppError::from)?;
    let sender = ActorSummary {
        user_id: claims.sub,
        username: "coffeelabs".to_string(),
        display_name: "Coffee Labs Push Test".to_string(),
        avatar_url: None,
    };
    let mut evt = build_chat_message_event(
        claims.sub,
        Uuid::now_v7(),
        sender,
        "Notifikasi chat pop-up ala WhatsApp aktif dan siap menerima pesan!",
    );
    evt.is_critical_transactional = true;
    state.notifications.publish_immediate(evt).await;

    Ok(ApiResponse::ok(
        "Notifikasi uji coba telah dikirim ke perangkat Anda.",
        json!({
            "fcm_server_configured": state.fcm_oauth.is_configured(),
            "active_device_tokens": tokens.len(),
        }),
    ))
}
