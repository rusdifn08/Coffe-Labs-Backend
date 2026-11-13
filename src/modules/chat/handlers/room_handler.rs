use axum::{
    extract::{Query, State},
    http::StatusCode,
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
        dto::{
            ChatPeerDto, ConversationListItemDto, CreateDirectRoomReq, CreateGroupRoomReq,
            PeerSearchQuery,
        },
        repositories::{conversation_repo, peer_repo},
    },
};

fn parse_claim_user_id(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub)
        .map_err(|_| AppError::Unauthorized("Format identitas pengguna tidak valid.".to_string()))
}

pub async fn list_conversations(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<ConversationListItemDto>>>> {
    let user_id = parse_claim_user_id(&claims.sub)?;
    let rows = conversation_repo::list_user_conversations(&state.pool, user_id)
        .await
        .map_err(AppError::from)?;

    let items = rows
        .into_iter()
        .map(|row| {
            let (peer_online, peer_last_seen_at) = row
                .peer_user_id
                .map(|uid| state.chat_presence.snapshot(uid))
                .unwrap_or((false, None));

            let is_direct = row.kind == "DIRECT";
            let display_title = if is_direct {
                row.peer_full_name
                    .clone()
                    .or_else(|| row.peer_username.clone())
                    .or_else(|| row.title.clone())
                    .unwrap_or_else(|| "Barista Coffee Labs".to_string())
            } else {
                row.title
                    .clone()
                    .unwrap_or_else(|| "Grup Coffee Labs".to_string())
            };

            let display_avatar = if is_direct {
                row.peer_avatar_url.or(row.avatar_url)
            } else {
                row.avatar_url
            };

            let unread_count = (row.last_seq_id - row.my_read_seq_id).max(0);

            ConversationListItemDto {
                id: row.id,
                kind: row.kind,
                title: display_title,
                avatar_url: display_avatar,
                last_seq_id: row.last_seq_id,
                unread_count,
                my_read_seq_id: row.my_read_seq_id,
                my_delivered_seq_id: row.my_delivered_seq_id,
                last_message_preview: row.last_message_preview,
                last_message_at: row.last_message_at,
                peer_user_id: row.peer_user_id,
                peer_username: row.peer_username,
                peer_online,
                peer_last_seen_at,
                peer_delivered_seq_id: row.peer_delivered_seq_id,
                peer_read_seq_id: row.peer_read_seq_id,
            }
        })
        .collect();

    Ok(ApiResponse::ok("Daftar percakapan berhasil dimuat.", items))
}

pub async fn create_direct_room(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateDirectRoomReq>,
) -> AppResult<(StatusCode, Json<ApiResponse<serde_json::Value>>)> {
    let user_id = parse_claim_user_id(&claims.sub)?;
    let conv_id = conversation_repo::find_or_create_direct_room(&state.pool, user_id, req.target_user_id)
        .await
        .map_err(AppError::from)?;
    state.chat_messages.invalidate_room(conv_id).await;

    Ok(ApiResponse::created(
        "Ruang direct chat siap digunakan.",
        json!({ "conversation_id": conv_id }),
    ))
}

pub async fn create_group_room(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateGroupRoomReq>,
) -> AppResult<(StatusCode, Json<ApiResponse<serde_json::Value>>)> {
    let user_id = parse_claim_user_id(&claims.sub)?;
    let conv_id = conversation_repo::create_group_room(
        &state.pool,
        user_id,
        &req.title,
        req.avatar_url.as_deref(),
        &req.member_ids,
    )
    .await
    .map_err(AppError::from)?;
    state.chat_messages.invalidate_room(conv_id).await;

    Ok(ApiResponse::created(
        "Grup Social Coffee berhasil dibuat.",
        json!({ "conversation_id": conv_id }),
    ))
}

pub async fn list_peers(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<PeerSearchQuery>,
) -> AppResult<Json<ApiResponse<Vec<ChatPeerDto>>>> {
    let user_id = parse_claim_user_id(&claims.sub)?;
    let rows = peer_repo::search_peers(
        &state.pool,
        user_id,
        query.q.as_deref(),
        query.limit.unwrap_or(25),
    )
    .await
    .map_err(AppError::from)?;

    let peers = rows
        .into_iter()
        .map(|r| {
            let (is_online, last_seen_at) = state.chat_presence.snapshot(r.id);
            ChatPeerDto {
                id: r.id,
                username: r.username,
                full_name: r.full_name,
                avatar_url: r.avatar_url,
                role: r.role,
                is_online,
                last_seen_at,
            }
        })
        .collect();

    Ok(ApiResponse::ok("Daftar barista ditemukan.", peers))
}
