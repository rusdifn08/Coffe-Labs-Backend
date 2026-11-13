use fred::{
    clients::{RedisPool, SubscriberClient},
    interfaces::{ClientLike, EventInterface, KeysInterface, PubsubInterface},
    types::Expiration,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    core::redis::RedisManager,
    modules::chat::{protocol::ServerFrame, services::connection_hub::ConnectionHub},
};

const USER_CHANNEL_PREFIX: &str = "social_coffee:user:";
const USER_CHANNEL_PATTERN: &str = "social_coffee:user:*";

/// Bridges local ConnectionHub with Redis Pub/Sub (`fred`) when `REDIS_URL` is configured,
/// gracefully operating in zero-overhead local in-memory mode on single-node deployments.
pub struct RedisPubSubBridge {
    hub: Arc<ConnectionHub>,
    redis_pool: Option<RedisPool>,
}

impl RedisPubSubBridge {
    #[allow(dead_code)]
    pub async fn connect_or_local(
        hub: Arc<ConnectionHub>,
        redis_url: Option<&str>,
    ) -> Arc<Self> {
        let Ok(manager) = RedisManager::connect_or_lazy(redis_url.unwrap_or_default()).await else {
            return Arc::new(Self {
                hub,
                redis_pool: None,
            });
        };
        Self::from_shared(hub, manager.active_pool(), manager.active_subscriber()).await
    }

    pub async fn from_shared(
        hub: Arc<ConnectionHub>,
        redis_pool: Option<RedisPool>,
        subscriber: Option<SubscriberClient>,
    ) -> Arc<Self> {
        let Some(pool) = redis_pool else {
            return Arc::new(Self {
                hub,
                redis_pool: None,
            });
        };

        if let Some(sub) = subscriber {
            spawn_subscriber_loop(hub.clone(), sub);
            info!("🔗 Redis Pub/Sub Bridge (`fred`) aktif menggunakan shared RedisPool & SubscriberClient.");
        } else {
            warn!("⚠️ SubscriberClient tidak tersedia; Redis Pub/Sub fan-out berjalan pada local hub fallback.");
        }

        Arc::new(Self {
            hub,
            redis_pool: Some(pool),
        })
    }

    pub async fn publish_to_user(&self, user_id: Uuid, frame: Arc<ServerFrame>) {
        if let Some(pool) = &self.redis_pool {
            if let Ok(payload) = serde_json::to_string(frame.as_ref()) {
                let channel = format!("{USER_CHANNEL_PREFIX}{user_id}");
                if pool.next().publish::<i64, _, _>(channel, payload).await.is_ok() {
                    return;
                }
            }
        }
        self.hub.send_local(user_id, frame);
    }

    pub async fn publish_to_users(&self, user_ids: &[Uuid], frame: Arc<ServerFrame>) {
        for &user_id in user_ids {
            self.publish_to_user(user_id, Arc::clone(&frame)).await;
        }
    }

    pub fn refresh_presence_lease(&self, user_id: Uuid, ttl_seconds: i64) {
        let Some(pool) = self.redis_pool.clone() else {
            return;
        };
        tokio::spawn(async move {
            let key = format!("presence:user:{user_id}");
            let _ = pool
                .set::<(), _, _>(key, "1", Some(Expiration::EX(ttl_seconds)), None, false)
                .await;
        });
    }

    pub fn redis_pool(&self) -> Option<&RedisPool> {
        self.redis_pool.as_ref()
    }

    pub fn clear_presence_lease(&self, user_id: Uuid) {
        let Some(pool) = self.redis_pool.clone() else {
            return;
        };
        tokio::spawn(async move {
            let key = format!("presence:user:{user_id}");
            let _ = pool.del::<i64, _>(key).await;
        });
    }
}

fn spawn_subscriber_loop(hub: Arc<ConnectionHub>, subscriber: SubscriberClient) {
    tokio::spawn(async move {
        let _keep_sub_alive = subscriber.clone();
        if let Err(err) = subscriber.wait_for_connect().await {
            warn!(error = %err, "⚠️ Gagal menunggu koneksi SubscriberClient Redis");
            return;
        }
        if let Err(err) = subscriber.psubscribe(USER_CHANNEL_PATTERN).await {
            warn!(error = %err, "⚠️ Gagal psubscribe channel Social Coffee Chat");
            return;
        }

        let mut message_rx = subscriber.message_rx();
        while let Ok(msg) = message_rx.recv().await {
            let channel = msg.channel.to_string();
            let Some(raw_uid) = channel.strip_prefix(USER_CHANNEL_PREFIX) else {
                continue;
            };
            let Ok(user_id) = Uuid::parse_str(raw_uid) else {
                continue;
            };
            let Some(json_str) = msg.value.as_string() else {
                continue;
            };
            if let Ok(frame) = serde_json::from_str::<ServerFrame>(&json_str) {
                hub.send_local(user_id, Arc::new(frame));
            }
        }
    });
}
