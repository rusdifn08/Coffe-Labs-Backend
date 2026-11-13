use moka::future::Cache;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    core::cache::{AppCaches, CHAT_MEMBER_CACHE_MAX_CAPACITY, CHAT_MEMBER_CACHE_TTL_SECS},
    modules::chat::{
        error::ChatError,
        protocol::{ChatMessageDto, InteractiveAttachment, MessageType, ServerFrame},
        repositories::{message_repo, peer_repo},
        services::{
            moderation_guard::{ModerationGuard, ModerationMode},
            redis_pubsub_bridge::RedisPubSubBridge,
        },
    },
};

pub struct SendMessageInput {
    pub sender_id: Uuid,
    pub client_msg_id: Uuid,
    pub conversation_id: Uuid,
    pub reply_to_msg_id: Option<Uuid>,
    pub message_type: MessageType,
    pub body_text: Option<String>,
    pub attachment: Option<InteractiveAttachment>,
}

/// Orchestrates participant checks (L1 moka 300s cache), persistence, acks & Pub/Sub fan-out.
pub struct MessageService {
    room_members_cache: Cache<Uuid, Arc<Vec<Uuid>>>,
    chat_member_l1: Cache<(Uuid, Uuid), bool>,
}

impl Default for MessageService {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageService {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_member_cache(AppCaches::build_chat_member_l1())
    }

    pub fn with_member_cache(chat_member_l1: Cache<(Uuid, Uuid), bool>) -> Self {
        let room_members_cache = Cache::builder()
            .max_capacity(CHAT_MEMBER_CACHE_MAX_CAPACITY)
            .time_to_live(std::time::Duration::from_secs(CHAT_MEMBER_CACHE_TTL_SECS))
            .build();
        Self {
            room_members_cache,
            chat_member_l1,
        }
    }

    pub async fn get_room_members(
        &self,
        pool: &PgPool,
        conversation_id: Uuid,
    ) -> Result<Arc<Vec<Uuid>>, ChatError> {
        if let Some(cached) = self.room_members_cache.get(&conversation_id).await {
            return Ok(cached);
        }
        let ids = peer_repo::get_participant_ids(pool, conversation_id).await?;
        let arc_ids = Arc::new(ids);
        self.room_members_cache
            .insert(conversation_id, Arc::clone(&arc_ids))
            .await;
        Ok(arc_ids)
    }

    pub async fn invalidate_room(&self, conversation_id: Uuid) {
        self.room_members_cache.invalidate(&conversation_id).await;
    }

    pub async fn verify_membership(
        &self,
        pool: &PgPool,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<Arc<Vec<Uuid>>, ChatError> {
        if self.chat_member_l1.get(&(conversation_id, user_id)).await == Some(false) {
            return Err(ChatError::NotParticipant);
        }
        let members = self.get_room_members(pool, conversation_id).await?;
        if members.contains(&user_id) {
            self.chat_member_l1
                .insert((conversation_id, user_id), true)
                .await;
            return Ok(members);
        }
        self.invalidate_room(conversation_id).await;
        let fresh = self.get_room_members(pool, conversation_id).await?;
        let allowed = fresh.contains(&user_id);
        self.chat_member_l1
            .insert((conversation_id, user_id), allowed)
            .await;
        if allowed {
            Ok(fresh)
        } else {
            Err(ChatError::NotParticipant)
        }
    }

    pub async fn process_send_message(
        &self,
        pool: &PgPool,
        bridge: &RedisPubSubBridge,
        moderation: &ModerationGuard,
        input: SendMessageInput,
    ) -> Result<ChatMessageDto, ChatError> {
        let members = self
            .verify_membership(pool, input.conversation_id, input.sender_id)
            .await?;

        let sanitized_text = match input.body_text.as_deref() {
            Some(raw) => {
                let clean = moderation.check_text(raw, ModerationMode::MaskWithAsterisks)?;
                if clean.is_empty() { None } else { Some(clean) }
            }
            None => None,
        };

        if sanitized_text.is_none() && input.attachment.is_none() {
            return Err(ChatError::InvalidFrame(
                "Pesan teks atau lampiran tidak boleh kosong.".to_string(),
            ));
        }

        let preview = build_preview_text(sanitized_text.as_deref(), input.attachment.as_ref());
        let saved = message_repo::persist_message_idempotent(
            pool,
            message_repo::PersistMessageParams {
                conversation_id: input.conversation_id,
                sender_id: input.sender_id,
                client_msg_id: input.client_msg_id,
                reply_to_msg_id: input.reply_to_msg_id,
                message_type: input.message_type,
                body_text: sanitized_text.as_deref(),
                attachment: input.attachment.as_ref(),
                preview_text: &preview,
            },
        )
        .await?;

        let dto = ChatMessageDto::from(saved);
        let ack_frame = Arc::new(ServerFrame::MessageSentAck {
            client_msg_id: dto.client_msg_id,
            server_msg_id: dto.id,
            conversation_id: dto.conversation_id,
            seq_id: dto.seq_id,
            created_at: dto.created_at,
        });
        bridge.publish_to_user(input.sender_id, ack_frame).await;

        let new_msg_frame = Arc::new(ServerFrame::NewMessage(dto.clone()));
        bridge.publish_to_users(members.as_slice(), new_msg_frame).await;

        Ok(dto)
    }
}

fn build_preview_text(
    body_text: Option<&str>,
    attachment: Option<&InteractiveAttachment>,
) -> String {
    let raw = match attachment {
        Some(InteractiveAttachment::V60RecipeCard { title, .. }) => format!("☕ Resep: {}", title),
        Some(InteractiveAttachment::CoffeeBeanCard { bean_name, roastery, .. }) => {
            format!("🫘 Biji: {} ({})", bean_name, roastery)
        }
        Some(InteractiveAttachment::BrewImageMedia { caption, .. }) => {
            caption.as_deref().unwrap_or("📷 Foto seduhan").to_string()
        }
        Some(InteractiveAttachment::BrewSessionResult { title, accuracy_score, .. }) => {
            format!("🏆 Seduh {}: {}%", title, accuracy_score)
        }
        None => body_text.unwrap_or("").to_string(),
    };
    raw.chars().take(240).collect()
}
