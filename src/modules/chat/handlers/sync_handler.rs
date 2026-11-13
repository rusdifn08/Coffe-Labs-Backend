use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::chat::{
        dto::{ChatHistoryResponseDto, DeltaSyncHttpReq, HistoryQuery},
        protocol::{ChatMessageDto, ParticipantWatermarkDto},
        repositories::message_repo,
        services::ack_sync_service,
    },
};

fn parse_claim_user_id(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub)
        .map_err(|_| AppError::Unauthorized("Format identitas pengguna tidak valid.".to_string()))
}

pub async fn get_conversation_history(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(conversation_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> AppResult<Json<ApiResponse<ChatHistoryResponseDto>>> {
    let user_id = parse_claim_user_id(&claims.sub)?;
    let members = state
        .chat_messages
        .get_room_members(&state.pool, conversation_id)
        .await
        .map_err(AppError::from)?;

    if !members.contains(&user_id) {
        return Err(AppError::Forbidden(
            "Anda bukan peserta ruang percakapan ini.".to_string(),
        ));
    }

    let req_limit = query.limit.unwrap_or(50).clamp(1, 100);
    let mut rows = message_repo::fetch_conversation_history(
        &state.pool,
        conversation_id,
        query.before_seq,
        req_limit + 1,
    )
    .await
    .map_err(AppError::from)?;

    let has_more = rows.len() > req_limit as usize;
    if has_more {
        rows.truncate(req_limit as usize);
    }

    let watermarks = message_repo::fetch_watermarks_for_user(
        &state.pool,
        user_id,
        &[conversation_id],
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Riwayat percakapan berhasil dimuat.",
        ChatHistoryResponseDto {
            conversation_id,
            messages: rows.into_iter().map(ChatMessageDto::from).collect(),
            watermarks: watermarks
                .into_iter()
                .map(ParticipantWatermarkDto::from)
                .collect(),
            has_more,
        },
    ))
}

pub async fn sync_delta_http(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<DeltaSyncHttpReq>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let user_id = parse_claim_user_id(&claims.sub)?;
    let (messages, watermarks) =
        ack_sync_service::compute_delta_sync(&state.pool, user_id, &req.cursors)
            .await
            .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Delta sinkronisasi pesan selesai.",
        json!({
            "messages": messages,
            "watermarks": watermarks,
        }),
    ))
}
