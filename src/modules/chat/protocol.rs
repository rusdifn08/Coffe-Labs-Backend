use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub use crate::modules::chat::attachments::{InteractiveAttachment, MessageType, ReceiptStatus};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversationCursorDto {
    pub conversation_id: Uuid,
    pub local_max_seq_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatMessageDto {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub seq_id: i64,
    pub sender_id: Uuid,
    pub client_msg_id: Uuid,
    pub reply_to_msg_id: Option<Uuid>,
    pub message_type: MessageType,
    pub body_text: Option<String>,
    pub attachment: Option<InteractiveAttachment>,
    pub is_edited: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ParticipantWatermarkDto {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub last_delivered_seq_id: i64,
    pub last_read_seq_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientFrame {
    SendMessage {
        client_msg_id: Uuid,
        conversation_id: Uuid,
        #[serde(default)]
        reply_to_msg_id: Option<Uuid>,
        message_type: MessageType,
        #[serde(default)]
        body_text: Option<String>,
        #[serde(default)]
        attachment: Option<InteractiveAttachment>,
    },
    AckDelivered {
        conversation_id: Uuid,
        up_to_seq_id: i64,
    },
    AckRead {
        conversation_id: Uuid,
        up_to_seq_id: i64,
    },
    Typing {
        conversation_id: Uuid,
        is_typing: bool,
    },
    SyncDelta {
        cursors: Vec<ConversationCursorDto>,
    },
    Ping {
        timestamp_ms: i64,
    },
    SetActiveRoom {
        conversation_id: Option<Uuid>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerFrame {
    MessageSentAck {
        client_msg_id: Uuid,
        server_msg_id: Uuid,
        conversation_id: Uuid,
        seq_id: i64,
        created_at: DateTime<Utc>,
    },
    NewMessage(ChatMessageDto),
    ReceiptWatermarkUpdate {
        conversation_id: Uuid,
        user_id: Uuid,
        status: ReceiptStatus,
        up_to_seq_id: i64,
        updated_at: DateTime<Utc>,
    },
    TypingIndicator {
        conversation_id: Uuid,
        user_id: Uuid,
        display_name: String,
        is_typing: bool,
    },
    PresenceUpdate {
        user_id: Uuid,
        is_online: bool,
        last_seen_at: Option<DateTime<Utc>>,
    },
    DeltaSyncBatch {
        messages: Vec<ChatMessageDto>,
        watermarks: Vec<ParticipantWatermarkDto>,
    },
    FrameError {
        client_msg_id: Option<Uuid>,
        code: String,
        message: String,
    },
    Pong {
        timestamp_ms: i64,
    },
    SocialGraphEvent {
        actor_id: Uuid,
        actor_username: String,
        actor_display_name: String,
        actor_avatar_url: Option<String>,
        state: String,
        action: String,
        message: String,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    PushNotificationEvent {
        notification_id: Uuid,
        category: String,
        event_type: String,
        title: String,
        body: String,
        deep_link_path: String,
        collapse_key: Option<String>,
        entity_id: Option<String>,
        actor_id: Option<Uuid>,
        conversation_id: Option<Uuid>,
        action_buttons: Vec<String>,
        unread_count: i64,
        created_at: DateTime<Utc>,
    },
}
