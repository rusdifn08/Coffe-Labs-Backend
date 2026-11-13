pub mod cache;
pub mod config;
pub mod database;
pub mod error;
pub mod extractors;
pub mod fcm_oauth;
pub mod middleware;
pub mod migrations;
pub mod pagination;
pub mod rabbitmq;
pub mod redis;
pub mod security;
pub mod seeds;
pub mod storage;
pub mod telemetry;

use axum::extract::FromRef;
use fred::clients::RedisPool;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager,
        middleware::SecurityRateLimiter, rabbitmq::RabbitMqClient, redis::RedisManager,
        security::JwtKeyRing, storage::CloudinaryStorage, telemetry::TelemetryRegistry,
    },
    modules::{
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        communities::{
            secret_id_crypto::derive_hmac_key,
            services::{
                club_cache_service::ClubCacheService, secret_join_service::SecretJoinService,
            },
        },
        notifications::services::producer_service::NotificationProducerService,
        platform::models::AppVersionConfig,
        recommendation::{
            taste_profiler_service::TasteProfilerService, trending_worker::spawn_trending_worker,
        },
        social_feed::services::profile_cqrs_service::ProfileCqrsService,
        social_graph::services::{
            graph_command_service::GraphCommandService,
            graph_query_service::GraphQueryService,
        },
    },
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
    pub jwt_keys: JwtKeyRing,
    pub telemetry: TelemetryRegistry,
    pub rate_limiter: SecurityRateLimiter,
    pub version_cache: Arc<RwLock<AppVersionConfig>>,
    pub chat_hub: Arc<ConnectionHub>,
    pub chat_bridge: Arc<RedisPubSubBridge>,
    pub chat_presence: Arc<PresenceService>,
    pub chat_messages: Arc<MessageService>,
    pub chat_moderation: Arc<ModerationGuard>,
    pub social_profile_cqrs: Arc<ProfileCqrsService>,
    pub social_graph_cmd: Arc<GraphCommandService>,
    pub social_graph_query: Arc<GraphQueryService>,
    pub notifications: Arc<NotificationProducerService>,
    pub rabbitmq: RabbitMqClient,
    pub redis: RedisManager,
    pub caches: AppCaches,
    pub fcm_oauth: FcmOAuthManager,
    pub cloudinary: CloudinaryStorage,
    pub club_cache: Arc<ClubCacheService>,
    pub secret_join_service: Arc<SecretJoinService>,
    pub taste_profiler: Arc<TasteProfilerService>,
}

pub fn init_module35_engines(
    pool: &PgPool,
    redis_pool: Option<RedisPool>,
    config: &AppConfig,
) -> (
    Arc<ClubCacheService>,
    Arc<SecretJoinService>,
    Arc<TasteProfilerService>,
) {
    let club_cache = Arc::new(ClubCacheService::new(redis_pool.clone()));
    let hmac_key = derive_hmac_key(config.jwt_seed.expose_secret());
    let secret_join = Arc::new(SecretJoinService::new(
        pool.clone(),
        hmac_key,
        club_cache.clone(),
    ));
    let taste_profiler = Arc::new(TasteProfilerService::new(pool.clone()));
    spawn_trending_worker(pool.clone(), redis_pool);
    (club_cache, secret_join, taste_profiler)
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for JwtKeyRing {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_keys.clone()
    }
}

impl FromRef<AppState> for TelemetryRegistry {
    fn from_ref(state: &AppState) -> Self {
        state.telemetry.clone()
    }
}

impl FromRef<AppState> for Arc<RwLock<AppVersionConfig>> {
    fn from_ref(state: &AppState) -> Self {
        state.version_cache.clone()
    }
}

impl FromRef<AppState> for RabbitMqClient {
    fn from_ref(state: &AppState) -> Self {
        state.rabbitmq.clone()
    }
}

impl FromRef<AppState> for RedisManager {
    fn from_ref(state: &AppState) -> Self {
        state.redis.clone()
    }
}

impl FromRef<AppState> for Option<RedisPool> {
    fn from_ref(state: &AppState) -> Self {
        state.redis.active_pool()
    }
}

impl FromRef<AppState> for AppCaches {
    fn from_ref(state: &AppState) -> Self {
        state.caches.clone()
    }
}

impl FromRef<AppState> for FcmOAuthManager {
    fn from_ref(state: &AppState) -> Self {
        state.fcm_oauth.clone()
    }
}

impl FromRef<AppState> for CloudinaryStorage {
    fn from_ref(state: &AppState) -> Self {
        state.cloudinary.clone()
    }
}
