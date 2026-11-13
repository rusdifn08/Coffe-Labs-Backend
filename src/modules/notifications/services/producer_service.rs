use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

use chrono::Utc;
use fred::interfaces::{HashesInterface, KeysInterface, SortedSetsInterface, StreamsInterface};
use fred::types::SetOptions;
use tokio::sync::{mpsc, Mutex};
use tracing::warn;
use uuid::Uuid;

use crate::modules::notifications::events::{
    format_coalesced_actor_names, ActorSummary, OmnichannelNotifEvent,
};

pub const NOTIF_STREAM_KEY: &str = "notif:stream:instant";
pub const NOTIF_DLQ_KEY: &str = "notif:stream:dlq";
pub const NOTIF_DELAYED_ZSET_KEY: &str = "notif:delayed_schedule";
pub const COALESCE_WINDOW_MS: i64 = 15_000;

struct LocalCoalesceBucket {
    template: OmnichannelNotifEvent,
    actors: HashMap<Uuid, ActorSummary>,
}

pub struct NotificationProducerService {
    redis: Option<fred::clients::RedisClient>,
    local_tx: mpsc::UnboundedSender<OmnichannelNotifEvent>,
    local_coalesce: Arc<Mutex<HashMap<String, LocalCoalesceBucket>>>,
}

impl NotificationProducerService {
    pub fn new(
        redis: Option<fred::clients::RedisClient>,
        local_tx: mpsc::UnboundedSender<OmnichannelNotifEvent>,
    ) -> Self {
        Self {
            redis,
            local_tx,
            local_coalesce: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn noop_for_tests() -> Arc<Self> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            while rx.recv().await.is_some() {}
        });
        Arc::new(Self::new(None, tx))
    }

    pub fn has_redis(&self) -> bool {
        self.redis.is_some()
    }

    /// Fire-and-forget publish ke Redis Streams / local worker channel agar HTTP handler tetap < 0.5ms.
    pub fn publish(self: &Arc<Self>, event: OmnichannelNotifEvent) {
        let this = Arc::clone(self);
        tokio::spawn(async move {
            this.publish_immediate(event).await;
        });
    }

    pub async fn publish_immediate(&self, event: OmnichannelNotifEvent) {
        if let Some(client) = &self.redis {
            if let Ok(payload) = serde_json::to_string(&event) {
                if let Err(err) = client
                    .xadd::<String, _, _, _, _>(
                        NOTIF_STREAM_KEY,
                        false,
                        ("MAXLEN", "~", crate::core::redis::REDIS_STREAM_MAXLEN as i64),
                        "*",
                        ("payload", payload),
                    )
                    .await
                {
                    warn!(error = %err, "Gagal XADD notif stream, fallback ke local worker");
                    let _ = self.local_tx.send(event);
                }
                return;
            }
        }
        let _ = self.local_tx.send(event);
    }

    /// Sliding-Window 15 detik untuk event noise tinggi (likes / recook).
    pub fn publish_coalesced(self: &Arc<Self>, event: OmnichannelNotifEvent) {
        let Some(coalesce_key) = event.coalesce_key.clone() else {
            self.publish(event);
            return;
        };

        let this = Arc::clone(self);
        tokio::spawn(async move {
            if let Some(client) = &this.redis {
                let hash_key = format!("notif:coalesce:{coalesce_key}");
                let mut fields: HashMap<String, String> = HashMap::new();
                if let Ok(meta) = serde_json::to_string(&event) {
                    fields.insert("__meta__".to_string(), meta);
                }
                if let Some(actor) = &event.actor {
                    if let Ok(actor_json) = serde_json::to_string(actor) {
                        fields.insert(actor.user_id.to_string(), actor_json);
                    }
                }
                if !fields.is_empty() {
                    let _ = client.hset::<i64, _, _>(&hash_key, fields).await;
                    let _ = client.expire::<i64, _>(&hash_key, 120).await;
                    let fire_at_ms = (Utc::now().timestamp_millis() + COALESCE_WINDOW_MS) as f64;
                    let _ = client
                        .zadd::<i64, _, _>(
                            NOTIF_DELAYED_ZSET_KEY,
                            Some(SetOptions::NX),
                            None,
                            false,
                            false,
                            (fire_at_ms, coalesce_key),
                        )
                        .await;
                    return;
                }
            }

            this.enqueue_local_coalesced(coalesce_key, event).await;
        });
    }

    async fn enqueue_local_coalesced(self: &Arc<Self>, key: String, event: OmnichannelNotifEvent) {
        let mut guard = self.local_coalesce.lock().await;
        let is_new = !guard.contains_key(&key);
        let bucket = guard.entry(key.clone()).or_insert_with(|| LocalCoalesceBucket {
            template: event.clone(),
            actors: HashMap::new(),
        });
        if let Some(actor) = event.actor.clone() {
            bucket.actors.insert(actor.user_id, actor);
        }
        drop(guard);

        if is_new {
            let this = Arc::clone(self);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(COALESCE_WINDOW_MS as u64)).await;
                let removed = {
                    let mut guard = this.local_coalesce.lock().await;
                    guard.remove(&key)
                };
                if let Some(bucket) = removed {
                    let actors: Vec<ActorSummary> = bucket.actors.into_values().collect();
                    let merged = merge_coalesced_event(bucket.template, &actors);
                    let _ = this.local_tx.send(merged);
                }
            });
        }
    }
}

pub fn merge_coalesced_event(
    mut template: OmnichannelNotifEvent,
    actors: &[ActorSummary],
) -> OmnichannelNotifEvent {
    let (label, preview) = format_coalesced_actor_names(actors);
    template.title = format!("{label} menyukai resep Anda");
    template.actors_preview = preview;
    template.aggregated_count = actors.len().max(1) as i32;
    template.coalesce_key = Some(
        template
            .coalesce_key
            .unwrap_or_else(|| format!("COALESCE:{}", template.event_id)),
    );
    template
}
