use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::{
    core::AppState,
    modules::{
        chat::{
            handlers::reply_handler,
            protocol::{ClientFrame, ServerFrame},
            services::{
                ack_sync_service, message_service::SendMessageInput,
                moderation_guard::SocketRateLimiter,
            },
        },
        notifications::handlers::device_handler::sync_active_room_redis,
    },
};

pub fn send_frame_error(
    state: &AppState,
    user_id: Uuid,
    client_msg_id: Option<Uuid>,
    code: &str,
    message: &str,
) {
    let err_frame = Arc::new(ServerFrame::FrameError {
        client_msg_id,
        code: code.to_string(),
        message: message.to_string(),
    });
    state.chat_hub.send_local(user_id, err_frame);
}

pub async fn dispatch_client_frame(
    state: &AppState,
    limiter: &SocketRateLimiter,
    user_id: Uuid,
    display_name: &str,
    frame: ClientFrame,
) {
    state
        .chat_presence
        .refresh_heartbeat(user_id, &state.chat_bridge);

    match frame {
        ClientFrame::SendMessage {
            client_msg_id,
            conversation_id,
            reply_to_msg_id,
            message_type,
            body_text,
            attachment,
        } => {
            if let Err(err) = limiter.check_message_quota() {
                send_frame_error(
                    state,
                    user_id,
                    Some(client_msg_id),
                    err.frame_error_code(),
                    &err.to_string(),
                );
                return;
            }
            let input = SendMessageInput {
                sender_id: user_id,
                client_msg_id,
                conversation_id,
                reply_to_msg_id,
                message_type,
                body_text,
                attachment,
            };
            match state
                .chat_messages
                .process_send_message(
                    &state.pool,
                    &state.chat_bridge,
                    &state.chat_moderation,
                    input,
                )
                .await
            {
                Ok(dto) => {
                    reply_handler::spawn_chat_push_notifications(state, user_id, &dto);
                }
                Err(err) => {
                    warn!(error = %err, "Gagal memproses SendMessage WS");
                    send_frame_error(
                        state,
                        user_id,
                        Some(client_msg_id),
                        err.frame_error_code(),
                        &err.to_string(),
                    );
                }
            }
        }
        ClientFrame::AckDelivered {
            conversation_id,
            up_to_seq_id,
        } => {
            let _ = ack_sync_service::process_watermark_ack(
                &state.chat_messages,
                &state.pool,
                &state.chat_bridge,
                conversation_id,
                user_id,
                up_to_seq_id,
                false,
            )
            .await;
        }
        ClientFrame::AckRead {
            conversation_id,
            up_to_seq_id,
        } => {
            let _ = ack_sync_service::process_watermark_ack(
                &state.chat_messages,
                &state.pool,
                &state.chat_bridge,
                conversation_id,
                user_id,
                up_to_seq_id,
                true,
            )
            .await;
        }
        ClientFrame::Typing {
            conversation_id,
            is_typing,
        } => {
            if limiter.check_typing_quota().is_err() {
                return;
            }
            let _ = ack_sync_service::process_typing_ephemeral(
                &state.chat_messages,
                &state.pool,
                &state.chat_bridge,
                conversation_id,
                user_id,
                display_name,
                is_typing,
            )
            .await;
        }
        ClientFrame::SyncDelta { cursors } => {
            match ack_sync_service::compute_delta_sync(&state.pool, user_id, &cursors).await {
                Ok((messages, watermarks)) => {
                    state.chat_hub.send_local(
                        user_id,
                        Arc::new(ServerFrame::DeltaSyncBatch {
                            messages,
                            watermarks,
                        }),
                    );
                }
                Err(err) => {
                    send_frame_error(state, user_id, None, err.frame_error_code(), &err.to_string());
                }
            }
        }
        ClientFrame::Ping { timestamp_ms } => {
            state
                .chat_hub
                .send_local(user_id, Arc::new(ServerFrame::Pong { timestamp_ms }));
        }
        ClientFrame::SetActiveRoom { conversation_id } => {
            sync_active_room_redis(state, user_id, conversation_id).await;
        }
    }
}
