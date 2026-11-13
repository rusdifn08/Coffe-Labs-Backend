use std::time::Duration;

use fred::{
    clients::{RedisClient, RedisPool, SubscriberClient},
    interfaces::ClientLike,
    types::{Builder, ReconnectPolicy, RedisConfig},
};
use tracing::{info, warn};

use crate::core::error::{AppError, AppResult};

/// Batas koneksi pool Redis Cloud Free Tier (maksimal 4 koneksi aktif + 1 subscriber).
pub const REDIS_POOL_SIZE: usize = 4;

/// Batas XADD MAXLEN pada Redis Streams agar memori Redis Cloud tetap di bawah 2 MB dari 30 MB Free Tier.
pub const REDIS_STREAM_MAXLEN: u64 = 1_000;

/// Default TTL L2 Redis profile materialized view (detik).
pub const PROFILE_CACHE_TTL_SECS: i64 = 300;

const RECONNECT_MAX_ATTEMPTS: u32 = 0;
const RECONNECT_MIN_DELAY_MS: u32 = 100;
const RECONNECT_MAX_DELAY_MS: u32 = 30_000;
const RECONNECT_MULTIPLIER: u32 = 2;
const REDIS_STARTUP_TIMEOUT_SECS: u64 = 6;

/// Kebijakan auto-reconnect exponential backoff (`0, 100ms, 30s max, faktor 2`).
pub fn build_reconnect_policy() -> ReconnectPolicy {
    ReconnectPolicy::new_exponential(
        RECONNECT_MAX_ATTEMPTS,
        RECONNECT_MIN_DELAY_MS,
        RECONNECT_MAX_DELAY_MS,
        RECONNECT_MULTIPLIER,
    )
}

/// Menutup username/password pada REDIS_URL sebelum ditulis ke log `tracing`.
pub fn sanitize_redis_url(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return "[REDACTED_REDIS_URL]".to_string();
    };
    if let Some((_, host_and_port)) = rest.rsplit_once('@') {
        format!("{scheme}://***:***@{host_and_port}")
    } else {
        format!("{scheme}://{rest}")
    }
}

/// Infrastruktur koneksi Redis terpusat (`RedisPool` max 4 koneksi + `SubscriberClient`).
#[derive(Clone)]
pub struct RedisManager {
    pub pool: RedisPool,
    pub subscriber: SubscriberClient,
    configured: bool,
}

impl RedisManager {
    fn build_unconnected_lazy() -> AppResult<Self> {
        let mut pool_builder = Builder::default_centralized();
        pool_builder.set_policy(build_reconnect_policy());
        let pool = pool_builder
            .build_pool(REDIS_POOL_SIZE)
            .map_err(|e| AppError::Internal(format!("Gagal build fallback RedisPool: {e}")))?;

        let mut sub_builder = Builder::default_centralized();
        sub_builder.set_policy(build_reconnect_policy());
        let subscriber = sub_builder
            .build_subscriber_client()
            .map_err(|e| AppError::Internal(format!("Gagal build fallback SubscriberClient: {e}")))?;

        Ok(Self {
            pool,
            subscriber,
            configured: false,
        })
    }

    #[allow(dead_code)]
    pub fn disabled() -> AppResult<Self> {
        Self::build_unconnected_lazy()
    }

    pub async fn connect_or_lazy(redis_url: &str) -> AppResult<Self> {
        let trimmed = redis_url.trim();
        if trimmed.is_empty() {
            info!("🧊 REDIS_URL belum disetel; Social Coffee berjalan dalam mode In-Memory Single-Node.");
            return Self::build_unconnected_lazy();
        }

        let safe_url = sanitize_redis_url(trimmed);
        let config = match RedisConfig::from_url(trimmed) {
            Ok(cfg) => cfg,
            Err(err) => {
                warn!(redis = %safe_url, error = %err, "⚠️ REDIS_URL tidak valid; fallback ke mode In-Memory.");
                return Self::build_unconnected_lazy();
            }
        };

        let mut pool_builder = Builder::from_config(config.clone());
        pool_builder.set_policy(build_reconnect_policy());
        let pool = match pool_builder.build_pool(REDIS_POOL_SIZE) {
            Ok(p) => p,
            Err(err) => {
                warn!(redis = %safe_url, error = %err, "⚠️ Gagal membangun RedisPool; fallback ke mode In-Memory.");
                return Self::build_unconnected_lazy();
            }
        };

        let mut sub_builder = Builder::from_config(config);
        sub_builder.set_policy(build_reconnect_policy());
        let subscriber = sub_builder
            .build_subscriber_client()
            .map_err(|e| AppError::Internal(format!("Gagal membangun SubscriberClient: {e}")))?;

        let _pool_task = ClientLike::connect(&pool);
        match tokio::time::timeout(
            Duration::from_secs(REDIS_STARTUP_TIMEOUT_SECS),
            pool.wait_for_connect(),
        )
        .await
        {
            Ok(Ok(())) => {
                info!(redis = %safe_url, pool_size = REDIS_POOL_SIZE, "🔗 Terhubung ke Redis (`fred` RedisPool max 4 koneksi + exponential reconnect).");
            }
            Ok(Err(err)) => {
                warn!(redis = %safe_url, error = %err, "⚠️ Koneksi awal Redis gagal; background ReconnectPolicy aktif tanpa mematikan server.");
            }
            Err(_) => {
                warn!(redis = %safe_url, "⚠️ Timeout jabat tangan Redis saat startup; background ReconnectPolicy tetap aktif.");
            }
        }

        let _sub_task = ClientLike::connect(&subscriber);
        let _ = tokio::time::timeout(
            Duration::from_secs(REDIS_STARTUP_TIMEOUT_SECS),
            subscriber.wait_for_connect(),
        )
        .await;
        let _ = subscriber.manage_subscriptions();

        Ok(Self {
            pool,
            subscriber,
            configured: true,
        })
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn is_connected(&self) -> bool {
        self.configured && self.pool.is_connected()
    }

    pub fn active_pool(&self) -> Option<RedisPool> {
        self.configured.then(|| self.pool.clone())
    }

    pub fn active_subscriber(&self) -> Option<SubscriberClient> {
        self.configured.then(|| self.subscriber.clone())
    }

    pub fn active_client(&self) -> Option<RedisClient> {
        self.configured.then(|| self.pool.next().clone())
    }
}
