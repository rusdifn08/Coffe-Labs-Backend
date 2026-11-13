use std::{sync::Arc, time::Duration};

use chrono::Utc;
use fred::prelude::{Expiration, KeysInterface, SetOptions};
use moka::future::Cache;
use uuid::Uuid;

use crate::modules::{
    chat::{protocol::ServerFrame, services::redis_pubsub_bridge::RedisPubSubBridge},
    notifications::{
        builders::{build_mutual_follback_event, build_new_follower_event},
        events::ActorSummary,
        services::producer_service::NotificationProducerService,
    },
    social_feed::services::profile_cqrs_service::ProfileCqrsService,
    social_graph::models::{ActorSummaryRow, RelationshipState},
};

const COOLDOWN_SECS: i64 = 3600;

pub struct FollowNotifDispatcher {
    bridge: Arc<RedisPubSubBridge>,
    profile_cqrs: Arc<ProfileCqrsService>,
    push_producer: Option<Arc<NotificationProducerService>>,
    local_cooldown: Cache<(Uuid, Uuid), ()>,
}

impl FollowNotifDispatcher {
    pub fn new(bridge: Arc<RedisPubSubBridge>, profile_cqrs: Arc<ProfileCqrsService>) -> Self {
        let local_cooldown = Cache::builder()
            .max_capacity(50_000)
            .time_to_live(Duration::from_secs(COOLDOWN_SECS as u64))
            .build();
        Self {
            bridge,
            profile_cqrs,
            push_producer: None,
            local_cooldown,
        }
    }

    pub fn with_push_producer(mut self, producer: Arc<NotificationProducerService>) -> Self {
        self.push_producer = Some(producer);
        self
    }

    pub async fn invalidate_pair_profiles(&self, actor_id: Uuid, target_id: Uuid) {
        self.profile_cqrs.invalidate_profile(actor_id).await;
        self.profile_cqrs.invalidate_profile(target_id).await;
    }

    pub async fn try_acquire_cooldown(&self, sender_id: Uuid, recipient_id: Uuid) -> bool {
        if let Some(redis) = self.bridge.redis_pool() {
            let key = format!("notif:follow_cooldown:{sender_id}:{recipient_id}");
            if let Ok(res) = redis
                .set::<Option<String>, _, _>(
                    &key,
                    "1",
                    Some(Expiration::EX(COOLDOWN_SECS)),
                    Some(SetOptions::NX),
                    false,
                )
                .await
            {
                return res.is_some();
            }
        }
        if self.local_cooldown.get(&(sender_id, recipient_id)).await.is_some() {
            return false;
        }
        self.local_cooldown.insert((sender_id, recipient_id), ()).await;
        true
    }

    pub async fn notify_relationship_event(
        &self,
        actor: &ActorSummaryRow,
        recipient_id: Uuid,
        action: &str,
        recipient_view_state: RelationshipState,
        message: &str,
    ) {
        self.invalidate_pair_profiles(actor.user_id, recipient_id).await;
        if !self.try_acquire_cooldown(actor.user_id, recipient_id).await {
            tracing::debug!(
                sender_id = %actor.user_id,
                recipient_id = %recipient_id,
                action = %action,
                "Suppressed duplicate follow notification due to 1h cooldown"
            );
            return;
        }

        let frame = Arc::new(ServerFrame::SocialGraphEvent {
            actor_id: actor.user_id,
            actor_username: actor.username.clone(),
            actor_display_name: actor.display_name.clone(),
            actor_avatar_url: actor.avatar_url.clone(),
            state: recipient_view_state.as_str().to_string(),
            action: action.to_string(),
            message: message.to_string(),
            created_at: Utc::now(),
        });
        self.bridge.publish_to_user(recipient_id, frame).await;

        if let Some(producer) = &self.push_producer {
            let push_actor = ActorSummary {
                user_id: actor.user_id,
                username: actor.username.clone(),
                display_name: actor.display_name.clone(),
                avatar_url: actor.avatar_url.clone(),
            };
            if action == "follback" || recipient_view_state == RelationshipState::Mutual {
                producer.publish(build_mutual_follback_event(recipient_id, push_actor));
            } else if matches!(action, "follow" | "follow_request") {
                let is_follback_opportunity =
                    recipient_view_state == RelationshipState::FollowedByTarget;
                producer.publish(build_new_follower_event(
                    recipient_id,
                    push_actor,
                    is_follback_opportunity,
                ));
            }
        }
    }
}
