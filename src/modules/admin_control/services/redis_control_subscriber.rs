use std::sync::Arc;

use fred::{
    clients::SubscriberClient,
    interfaces::{ClientLike, EventInterface, PubsubInterface},
};
use tracing::warn;

use crate::modules::admin_control::{
    dto::ControlSnapshotDto,
    services::runtime_snapshot_store::{RuntimeSnapshotStore, REDIS_CONTROL_CHANNEL},
};

/// Menyambungkan listener Redis Pub/Sub untuk replikasi snapshot antar-instance Axum.
pub fn spawn_redis_control_subscriber(
    store: Arc<RuntimeSnapshotStore>,
    subscriber: SubscriberClient,
) {
    tokio::spawn(async move {
        let _keep_alive = subscriber.clone();
        if let Err(err) = subscriber.wait_for_connect().await {
            warn!(error = %err, "⚠️ Redis control subscriber gagal konek.");
            return;
        }
        if let Err(err) = subscriber.subscribe(REDIS_CONTROL_CHANNEL).await {
            warn!(error = %err, "⚠️ Gagal subscribe sys:control:broadcast.");
            return;
        }

        let mut rx = subscriber.message_rx();
        while let Ok(msg) = rx.recv().await {
            if msg.channel.to_string() != REDIS_CONTROL_CHANNEL {
                continue;
            }
            let Some(raw_json) = msg.value.as_string() else {
                continue;
            };
            if let Ok(incoming) = serde_json::from_str::<ControlSnapshotDto>(&raw_json) {
                let current_ver = store.load_snapshot().config_version;
                if incoming.config_version > current_ver {
                    store.apply_and_broadcast_local(incoming);
                }
            }
        }
    });
}
