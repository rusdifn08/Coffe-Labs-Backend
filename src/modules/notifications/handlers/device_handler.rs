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
        dto::{ActiveChatRoomReq, RegisterDeviceTokenReq, UnregisterDeviceReq, UpdateKioskModeReq},
        repositories::device_token_repo,
    },
};

fn parse_uid(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::Unauthorized("Identitas JWT tidak valid.".to_string()))
}

pub async fn put_register_device_token(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterDeviceTokenReq>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let row = device_token_repo::upsert_device_token(&state.pool, user_id, &req)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Token perangkat push berhasil didaftarkan.", row))
}

pub async fn patch_kiosk_mode(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<UpdateKioskModeReq>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    device_token_repo::update_kiosk_aod_flag(
        &state.pool,
        user_id,
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
    let user_id = parse_uid(&claims.sub)?;
    device_token_repo::deactivate_by_installation(&state.pool, user_id, &req.device_installation_id)
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
    let user_id = parse_uid(&claims.sub)?;
    sync_active_room_redis(&state, user_id, req.conversation_id).await;
    Ok(ApiResponse::ok(
        "Presensi ruang obrolan aktif diperbarui.",
        json!({ "conversation_id": req.conversation_id }),
    ))
}
