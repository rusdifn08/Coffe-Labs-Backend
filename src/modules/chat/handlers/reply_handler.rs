use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::AuthUser,
        AppState,
    },
    modules::{
        chat::{
            protocol::{ChatMessageDto, InteractiveAttachment, MessageType},
            services::message_service::SendMessageInput,
        },
        notifications::{
            builders::build_chat_message_event,
            repositories::actor_lookup,
        },
    },
};

#[derive(Debug, Deserialize)]
pub struct SendHttpMessageReq {
    pub client_msg_id: Option<Uuid>,
    pub reply_to_msg_id: Option<Uuid>,
    pub message_type: Option<MessageType>,
    pub body_text: Option<String>,
    pub attachment: Option<InteractiveAttachment>,
}

pub async fn post_send_message(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<SendHttpMessageReq>,
) -> AppResult<(StatusCode, Json<ApiResponse<ChatMessageDto>>)> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("Format identitas pengguna tidak valid.".to_string()))?;

    let input = SendMessageInput {
        sender_id: user_id,
        client_msg_id: req.client_msg_id.unwrap_or_else(Uuid::now_v7),
        conversation_id,
        reply_to_msg_id: req.reply_to_msg_id,
        message_type: req.message_type.unwrap_or(MessageType::Text),
        body_text: req.body_text,
        attachment: req.attachment,
    };

    let dto = state
        .chat_messages
        .process_send_message(
            &state.pool,
            &state.chat_bridge,
            &state.chat_moderation,
            input,
        )
        .await
        .map_err(AppError::from)?;

    spawn_chat_push_notifications(&state, user_id, &dto);

    Ok(ApiResponse::created("Pesan berhasil dikirim.", dto))
}

pub fn spawn_chat_push_notifications(state: &AppState, sender_id: Uuid, dto: &ChatMessageDto) {
    let pool = state.pool.clone();
    let messages = state.chat_messages.clone();
    let producer = state.notifications.clone();
    let conversation_id = dto.conversation_id;
    let preview = dto
        .body_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Mengirim lampiran seduh baru")
        .to_string();

    tokio::spawn(async move {
        let Ok(members) = messages.get_room_members(&pool, conversation_id).await else {
            return;
        };
        let Some(sender) = actor_lookup::lookup_actor_summary(&pool, sender_id).await else {
            return;
        };
        for &recipient_id in members.iter() {
            if recipient_id == sender_id {
                continue;
            }
            producer.publish(build_chat_message_event(
                recipient_id,
                conversation_id,
                sender.clone(),
                &preview,
            ));
        }
    });
}
