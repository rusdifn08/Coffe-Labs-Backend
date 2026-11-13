use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::chat::{
    error::ChatError,
    protocol::{
        ChatMessageDto, ConversationCursorDto, ParticipantWatermarkDto, ReceiptStatus, ServerFrame,
    },
    repositories::{message_repo, peer_repo},
    services::{
        message_service::MessageService, redis_pubsub_bridge::RedisPubSubBridge,
    },
};

pub async fn process_watermark_ack(
    service: &MessageService,
    pool: &PgPool,
    bridge: &RedisPubSubBridge,
    conversation_id: Uuid,
    user_id: Uuid,
    up_to_seq_id: i64,
    is_read: bool,
) -> Result<(), ChatError> {
    let members = service.verify_membership(pool, conversation_id, user_id).await?;
    let updated = peer_repo::advance_watermark(
        pool,
        conversation_id,
        user_id,
        up_to_seq_id,
        is_read,
    )
    .await?;

    let status = if is_read {
        ReceiptStatus::Read
    } else {
        ReceiptStatus::Delivered
    };
    let effective_seq = if is_read {
        updated.last_read_seq_id
    } else {
        updated.last_delivered_seq_id
    };

    let frame = Arc::new(ServerFrame::ReceiptWatermarkUpdate {
        conversation_id,
        user_id,
        status,
        up_to_seq_id: effective_seq,
        updated_at: Utc::now(),
    });

    let others: Vec<Uuid> = members.iter().copied().filter(|id| *id != user_id).collect();
    bridge.publish_to_users(&others, frame).await;
    Ok(())
}

pub async fn process_typing_ephemeral(
    service: &MessageService,
    pool: &PgPool,
    bridge: &RedisPubSubBridge,
    conversation_id: Uuid,
    user_id: Uuid,
    display_name: &str,
    is_typing: bool,
) -> Result<(), ChatError> {
    let members = service.verify_membership(pool, conversation_id, user_id).await?;
    let others: Vec<Uuid> = members.iter().copied().filter(|id| *id != user_id).collect();
    let frame = Arc::new(ServerFrame::TypingIndicator {
        conversation_id,
        user_id,
        display_name: display_name.to_string(),
        is_typing,
    });
    bridge.publish_to_users(&others, frame).await;
    Ok(())
}

pub async fn compute_delta_sync(
    pool: &PgPool,
    user_id: Uuid,
    cursors: &[ConversationCursorDto],
) -> Result<(Vec<ChatMessageDto>, Vec<ParticipantWatermarkDto>), ChatError> {
    let conv_ids: Vec<Uuid> = cursors.iter().map(|c| c.conversation_id).collect();
    let msgs = message_repo::fetch_delta_messages(pool, user_id, cursors).await?;
    let wms = message_repo::fetch_watermarks_for_user(pool, user_id, &conv_ids).await?;
    Ok((
        msgs.into_iter().map(ChatMessageDto::from).collect(),
        wms.into_iter().map(ParticipantWatermarkDto::from).collect(),
    ))
}
