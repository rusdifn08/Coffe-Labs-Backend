use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::modules::chat::protocol::{
    ChatMessageDto, ConversationCursorDto, ParticipantWatermarkDto,
};

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateDirectRoomReq {
    pub target_user_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct CreateGroupRoomReq {
    #[validate(length(min = 2, max = 120, message = "Nama grup harus 2-120 karakter."))]
    pub title: String,
    pub avatar_url: Option<String>,
    #[validate(length(min = 1, max = 50, message = "Pilih 1-50 anggota grup."))]
    pub member_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct DeltaSyncHttpReq {
    #[validate(length(max = 200, message = "Maksimal 200 percakapan per delta sync."))]
    pub cursors: Vec<ConversationCursorDto>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub before_seq: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PeerSearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct WsConnectQuery {
    pub token: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationListItemDto {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    pub avatar_url: Option<String>,
    pub last_seq_id: i64,
    pub unread_count: i64,
    pub my_read_seq_id: i64,
    pub my_delivered_seq_id: i64,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub peer_user_id: Option<Uuid>,
    pub peer_username: Option<String>,
    pub peer_online: bool,
    pub peer_last_seen_at: Option<DateTime<Utc>>,
    pub peer_delivered_seq_id: i64,
    pub peer_read_seq_id: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatHistoryResponseDto {
    pub conversation_id: Uuid,
    pub messages: Vec<ChatMessageDto>,
    pub watermarks: Vec<ParticipantWatermarkDto>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatPeerDto {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
}
