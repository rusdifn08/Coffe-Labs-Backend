use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::modules::{
    chat::{
        protocol::ServerFrame,
        services::{presence_service::PresenceService, redis_pubsub_bridge::RedisPubSubBridge},
    },
    notifications::{
        error::{NotifError, NotifResult},
        events::{NotificationCategory, OmnichannelNotifEvent},
        repositories::{device_token_repo, inbox_repo},
        services::{
            backoff::{is_event_enabled_for_user, is_quiet_hours_now},
            fcm_v1_gateway::{FcmDeliveryOutcome, FcmV1Gateway},
        },
    },
};

pub struct OmnichannelRouter {
    pool: PgPool,
    chat_bridge: Arc<RedisPubSubBridge>,
    presence: Arc<PresenceService>,
    fcm: Arc<FcmV1Gateway>,
    redis: Option<fred::clients::RedisClient>,
}

impl OmnichannelRouter {
    pub fn new(
        pool: PgPool,
        chat_bridge: Arc<RedisPubSubBridge>,
        presence: Arc<PresenceService>,
        fcm: Arc<FcmV1Gateway>,
        redis: Option<fred::clients::RedisClient>,
    ) -> Self {
        Self {
            pool,
            chat_bridge,
            presence,
            fcm,
            redis,
        }
    }

    async fn user_is_viewing_chat_room(&self, user_id: Uuid, room_id: Uuid) -> bool {
        if self.presence.active_room(user_id) == Some(room_id) {
            return true;
        }
        if let Some(client) = &self.redis {
            use fred::interfaces::KeysInterface;
            let key = format!("chat:active_room:{user_id}");
            if let Ok(Some(val)) = client.get::<Option<String>, _>(key).await {
                if let Ok(active) = Uuid::parse_str(val.trim()) {
                    return active == room_id;
                }
            }
        }
        false
    }

    pub async fn route_event(&self, event: &OmnichannelNotifEvent) -> NotifResult<()> {
        let pref =
            device_token_repo::get_or_default_preferences(&self.pool, event.target_user_id).await?;
        let enabled = is_event_enabled_for_user(&pref, event.category, &event.event_type);
        if !enabled && !event.is_critical_transactional {
            debug!(
                user_id = %event.target_user_id,
                event_type = %event.event_type,
                "Notifikasi dibatalkan oleh preferensi kategori pengguna"
            );
            return Ok(());
        }

        // 1. Selalu simpan ke Notification Inbox permanen & siarkan live WS frame
        let inbox_row = inbox_repo::insert_or_upsert_inbox(&self.pool, event).await?;
        let unread_count = inbox_repo::count_unread(&self.pool, event.target_user_id).await?;
        let buttons: Vec<String> = event
            .action_buttons
            .iter()
            .map(|b| b.wire_str().to_string())
            .collect();

        let ws_frame = ServerFrame::PushNotificationEvent {
            notification_id: inbox_row.id,
            category: event.category.as_str().to_string(),
            event_type: event.event_type.clone(),
            title: inbox_row.title,
            body: inbox_row.body,
            deep_link_path: event.deep_link_path.clone(),
            collapse_key: event.collapse_key.clone(),
            entity_id: event.entity_id.clone(),
            actor_id: event.actor.as_ref().map(|a| a.user_id),
            conversation_id: event.conversation_id,
            action_buttons: buttons,
            unread_count,
            created_at: inbox_row.updated_at,
        };
        self.chat_bridge
            .publish_to_user(event.target_user_id, Arc::new(ws_frame))
            .await;

        // 2. Active-Room Suppression: jangan bunyikan OS push jika room chat sedang terbuka di layar
        if event.category == NotificationCategory::Chat {
            if let Some(room_id) = event.conversation_id {
                if self
                    .user_is_viewing_chat_room(event.target_user_id, room_id)
                    .await
                {
                    return Ok(());
                }
            }
        }

        // 3. Quiet Hours Suppression (non-transaksional)
        if !event.is_critical_transactional && is_quiet_hours_now(&pref, Utc::now()) {
            return Ok(());
        }

        // 4. Multi-Device Fan-out dengan AOD Kiosk Lock Guard
        let tokens = device_token_repo::list_active_tokens(&self.pool, event.target_user_id).await?;
        let mut retry_candidate: Option<(Option<u64>, String)> = None;

        for device in tokens {
            if device.is_kiosk_aod_active {
                continue;
            }
            match self.fcm.send_to_token(&device.fcm_token, event).await {
                FcmDeliveryOutcome::Sent | FcmDeliveryOutcome::UnconfiguredDryRun => {}
                FcmDeliveryOutcome::PruneStaleToken => {
                    let _ = device_token_repo::deactivate_stale_token(&self.pool, &device.fcm_token)
                        .await;
                }
                FcmDeliveryOutcome::Retryable {
                    retry_after_secs,
                    reason,
                } => {
                    let _ =
                        device_token_repo::record_delivery_failure(&self.pool, &device.fcm_token)
                            .await;
                    if retry_candidate.is_none() {
                        retry_candidate = Some((retry_after_secs, reason));
                    }
                }
                FcmDeliveryOutcome::PermanentReject { status, body } => {
                    warn!(status, body = %body, "FCM permanent reject untuk perangkat pengguna");
                    let _ = device_token_repo::deactivate_stale_token(&self.pool, &device.fcm_token)
                        .await;
                }
            }
        }

        if let Some((retry_after_secs, reason)) = retry_candidate {
            return Err(NotifError::RetryableDelivery {
                retry_after_secs,
                reason,
            });
        }

        Ok(())
    }
}
