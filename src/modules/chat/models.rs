use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::modules::chat::protocol::{
    ChatMessageDto, InteractiveAttachment, MessageType, ParticipantWatermarkDto,
};

#[derive(Debug, Clone, FromRow)]
pub struct ConversationSummaryRow {
    pub id: Uuid,
    pub kind: String,
    pub title: Option<String>,
    pub avatar_url: Option<String>,
    pub last_seq_id: i64,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub my_read_seq_id: i64,
    pub my_delivered_seq_id: i64,
    pub peer_user_id: Option<Uuid>,
    pub peer_username: Option<String>,
    pub peer_full_name: Option<String>,
    pub peer_avatar_url: Option<String>,
    pub peer_read_seq_id: i64,
    pub peer_delivered_seq_id: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct ParticipantWatermarkRow {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub last_delivered_seq_id: i64,
    pub last_read_seq_id: i64,
}

impl From<ParticipantWatermarkRow> for ParticipantWatermarkDto {
    fn from(r: ParticipantWatermarkRow) -> Self {
        Self {
            conversation_id: r.conversation_id,
            user_id: r.user_id,
            last_delivered_seq_id: r.last_delivered_seq_id,
            last_read_seq_id: r.last_read_seq_id,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ChatMessageRow {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub seq_id: i64,
    pub sender_id: Uuid,
    pub client_msg_id: Uuid,
    pub reply_to_msg_id: Option<Uuid>,
    pub message_type: String,
    pub body_text: Option<String>,
    pub attachment_payload: Option<serde_json::Value>,
    pub is_edited: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

impl From<ChatMessageRow> for ChatMessageDto {
    fn from(r: ChatMessageRow) -> Self {
        let attachment = r
            .attachment_payload
            .and_then(|val| serde_json::from_value::<InteractiveAttachment>(val).ok());
        Self {
            id: r.id,
            conversation_id: r.conversation_id,
            seq_id: r.seq_id,
            sender_id: r.sender_id,
            client_msg_id: r.client_msg_id,
            reply_to_msg_id: r.reply_to_msg_id,
            message_type: MessageType::from_db_str(&r.message_type),
            body_text: r.body_text,
            attachment,
            is_edited: r.is_edited,
            is_deleted: r.is_deleted,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ChatPeerRow {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
}
