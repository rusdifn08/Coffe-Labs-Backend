use std::sync::{Arc, OnceLock};

use dashmap::DashMap;
use fred::clients::RedisPool;
use fred::interfaces::PubsubInterface;
use tokio::sync::broadcast;
use tracing::warn;
use uuid::Uuid;

use crate::modules::qris_billing::dto::QrisWsFrame;

const REDIS_QRIS_EVENT_CHANNEL: &str = "qris:billing:ws_events";

static GLOBAL_QRIS_HUB: OnceLock<Arc<QrisWsHub>> = OnceLock::new();

pub fn global_qris_ws_hub() -> Arc<QrisWsHub> {
    Arc::clone(GLOBAL_QRIS_HUB.get_or_init(|| Arc::new(QrisWsHub::new())))
}

/// Hub WebSocket berbasis DashMap + Redis Pub/Sub untuk siaran status pelunasan QRIS (< 2 detik).
pub struct QrisWsHub {
    channels: DashMap<Uuid, broadcast::Sender<QrisWsFrame>>,
}

impl QrisWsHub {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
        }
    }

    pub fn subscribe(&self, invoice_id: Uuid) -> broadcast::Receiver<QrisWsFrame> {
        let sender = self
            .channels
            .entry(invoice_id)
            .or_insert_with(|| broadcast::channel(32).0)
            .clone();
        sender.subscribe()
    }

    pub fn broadcast_local(&self, invoice_id: Uuid, frame: QrisWsFrame) {
        if let Some(tx) = self.channels.get(&invoice_id) {
            let _ = tx.send(frame);
        }
    }

    /// Menyiarkan frame ke koneksi lokal dan Redis Pub/Sub agar tetap sinkron pada multi-instance.
    pub async fn publish(&self, redis_pool: Option<&RedisPool>, invoice_id: Uuid, frame: QrisWsFrame) {
        self.broadcast_local(invoice_id, frame.clone());
        let Some(pool) = redis_pool else {
            return;
        };
        if let Ok(serialized) = serde_json::to_string(&frame) {
            if let Err(err) = pool
                .next()
                .publish::<i64, _, _>(REDIS_QRIS_EVENT_CHANNEL, serialized)
                .await
            {
                warn!(error = %err, %invoice_id, "Gagal menerbitkan event QRIS ke Redis Pub/Sub");
            }
        }
    }

    pub fn cleanup_idle_channel(&self, invoice_id: Uuid) {
        let should_remove = self
            .channels
            .get(&invoice_id)
            .map(|tx| tx.receiver_count() == 0)
            .unwrap_or(false);
        if should_remove {
            self.channels.remove(&invoice_id);
        }
    }
}

impl Default for QrisWsHub {
    fn default() -> Self {
        Self::new()
    }
}
