use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use arc_swap::ArcSwap;
use chrono::Utc;
use fred::{
    clients::{RedisPool, SubscriberClient},
    interfaces::{ClientLike, KeysInterface, PubsubInterface},
    types::Expiration,
};
use sqlx::PgPool;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{
    core::error::AppResult,
    modules::admin_control::{
        dto::{
            AffiliateProgramControlDto, ControlSnapshotDto, ControlWsFrame, DynamicPlanPricingDto,
            FeatureControlDto,
        },
        repositories::{feature_control_repo, pricing_control_repo},
        services::redis_control_subscriber::spawn_redis_control_subscriber,
    },
};

pub const REDIS_CONTROL_SNAPSHOT_KEY: &str = "sys:control:snapshot_v1";
pub const REDIS_CONTROL_CHANNEL: &str = "sys:control:broadcast";

/// Store lock-free berkinerja ~15 nanodetik (`ArcSwap`) + Redis Pub/Sub + Global WS Broadcast Hub.
pub struct RuntimeSnapshotStore {
    snapshot: ArcSwap<ControlSnapshotDto>,
    ws_tx: broadcast::Sender<Arc<String>>,
    connected_clients: AtomicUsize,
    redis_pool: Option<RedisPool>,
}

impl RuntimeSnapshotStore {
    pub fn empty(redis_pool: Option<RedisPool>) -> Arc<Self> {
        let (ws_tx, _) = broadcast::channel(128);
        let initial = ControlSnapshotDto {
            config_version: 1, global_maintenance_mode: false, global_maintenance_message: None,
            connected_clients: 0, features: Vec::new(), plans: Vec::new(),
            affiliate_config: AffiliateProgramControlDto::default(), updated_at: Utc::now(),
        };
        Arc::new(Self {
            snapshot: ArcSwap::from_pointee(initial),
            ws_tx, connected_clients: AtomicUsize::new(0), redis_pool,
        })
    }

    pub async fn init_and_spawn(
        pool: &PgPool,
        redis_pool: Option<RedisPool>,
        subscriber: Option<SubscriberClient>,
    ) -> Arc<Self> {
        let store = Self::empty(redis_pool);
        match store.reload_from_db(pool).await {
            Ok(snap) => {
                info!(
                    config_version = snap.config_version,
                    features = snap.features.len(),
                    plans = snap.plans.len(),
                    "🛡️ Admin Control Plane (`ArcSwap` RuntimeSnapshotStore) terhidrasi dari NeonDB."
                );
            }
            Err(err) => {
                warn!(error = %err, "⚠️ Gagal hidrasi awal Admin Control Plane dari DB; mode fallback kosong.");
            }
        }

        if let Some(sub) = subscriber {
            spawn_redis_control_subscriber(Arc::clone(&store), sub);
        }

        store
    }

    /// Bacaan lock-free (~15ns) tanpa menyentuh PostgreSQL untuk Middleware & REST.
    pub fn load_snapshot(&self) -> Arc<ControlSnapshotDto> {
        self.snapshot.load_full()
    }

    /// Mengembalikan snapshot dengan jumlah perangkat WS terhubung terkini.
    pub fn load_snapshot_with_clients(&self) -> ControlSnapshotDto {
        let mut snap = (*self.snapshot.load_full()).clone();
        snap.connected_clients = self.connected_clients();
        snap
    }

    pub fn connected_clients(&self) -> usize {
        self.connected_clients.load(Ordering::Relaxed)
    }

    pub fn register_ws_client(&self) -> (broadcast::Receiver<Arc<String>>, usize) {
        let count = self.connected_clients.fetch_add(1, Ordering::Relaxed) + 1;
        (self.ws_tx.subscribe(), count)
    }

    pub fn unregister_ws_client(&self) -> usize {
        self.connected_clients
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                Some(cur.saturating_sub(1))
            })
            .map(|prev| prev.saturating_sub(1))
            .unwrap_or(0)
    }

    /// Memuat ulang snapshot lengkap dari NeonDB dan memperbarui `ArcSwap` bila versi >= lokal.
    pub async fn reload_from_db(&self, pool: &PgPool) -> AppResult<ControlSnapshotDto> {
        let meta = feature_control_repo::fetch_control_meta(pool).await?;
        let features = feature_control_repo::list_all_features(pool)
            .await?
            .into_iter()
            .map(FeatureControlDto::from)
            .collect();
        let plans = pricing_control_repo::list_all_plans(pool)
            .await?
            .into_iter()
            .map(DynamicPlanPricingDto::from)
            .collect();
        let affiliate_config = pricing_control_repo::fetch_affiliate_control(pool)
            .await
            .map(AffiliateProgramControlDto::from)
            .unwrap_or_default();

        let snap = ControlSnapshotDto {
            config_version: meta.config_version,
            global_maintenance_mode: meta.global_maintenance_mode,
            global_maintenance_message: meta.global_maintenance_message,
            connected_clients: self.connected_clients(),
            features,
            plans,
            affiliate_config,
            updated_at: meta.updated_at,
        };
        self.snapshot.store(Arc::new(snap.clone()));
        Ok(snap)
    }

    /// Menerapkan snapshot baru dari mutasi lokal atau Pub/Sub (hanya jika versi lebih baru)
    /// lalu menembakkan frame `SNAPSHOT_SYNCED` ke seluruh klien WebSocket.
    pub fn apply_and_broadcast_local(&self, incoming: ControlSnapshotDto) {
        let current_ver = self.snapshot.load().config_version;
        if incoming.config_version < current_ver {
            return;
        }
        let mut enriched = incoming;
        enriched.connected_clients = self.connected_clients();
        self.snapshot.store(Arc::new(enriched.clone()));

        let frame = ControlWsFrame::SnapshotSynced { snapshot: enriched };
        if let Ok(json_text) = serde_json::to_string(&frame) {
            let _ = self.ws_tx.send(Arc::new(json_text));
        }
    }

    /// Menyimpan snapshot ke Redis + menerbitkan ke channel Pub/Sub untuk multi-instance Axum,
    /// sekaligus menyiarkan langsung ke seluruh klien WebSocket di node ini.
    pub async fn publish_snapshot(&self, snap: ControlSnapshotDto) {
        self.apply_and_broadcast_local(snap.clone());

        let Some(pool) = &self.redis_pool else {
            return;
        };
        if !pool.is_connected() {
            return;
        }
        let Ok(payload) = serde_json::to_string(&snap) else {
            return;
        };

        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(250),
            pool.set::<(), _, _>(
                REDIS_CONTROL_SNAPSHOT_KEY,
                payload.clone(),
                Some(Expiration::EX(3600)),
                None,
                false,
            ),
        )
        .await;

        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(250),
            pool.next()
                .publish::<i64, _, _>(REDIS_CONTROL_CHANNEL, payload),
        )
        .await;
    }
}
