mod app;
mod core;
mod modules;

use mimalloc::MiMalloc;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    core::{
        cache::AppCaches,
        config::AppConfig,
        database::init_db,
        fcm_oauth::FcmOAuthManager,
        middleware::SecurityRateLimiter,
        rabbitmq::{spawn_rabbitmq_worker, RabbitMqClient},
        redis::RedisManager,
        security::JwtKeyRing,
        storage::CloudinaryStorage,
        telemetry::TelemetryRegistry,
        AppState,
    },
    modules::{
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        notifications::{
            services::{
                fcm_v1_gateway::FcmV1Gateway,
                omnichannel_router::OmnichannelRouter,
                producer_service::NotificationProducerService,
            },
            workers::spawn_notification_workers,
        },
        platform::repository::fetch_version_config,
        social_feed::services::profile_cqrs_service::ProfileCqrsService,
    },
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,coffe_labs_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let port = config.port;

    info!("☕ Memulai Coffee Labs Enterprise Rust Axum Backend v2.0.0 (mimalloc)...");

    let pool = init_db(&config.database_url).await?;
    let jwt_keys = JwtKeyRing::from_seed(&config.jwt_seed)?;
    let initial_version = fetch_version_config(&pool).await;
    let version_cache = Arc::new(RwLock::new(initial_version));
    let telemetry = TelemetryRegistry::new();
    let rate_limiter = SecurityRateLimiter::new(
        config.rate_limit_per_second,
        config.rate_limit_burst,
    );

    let caches = AppCaches::new();
    let redis = RedisManager::connect_or_lazy(&config.redis_url).await?;
    let fcm_oauth = FcmOAuthManager::from_config(
        config.google_application_credentials.as_deref(),
        &config.fcm_project_id,
    );
    let cloudinary = CloudinaryStorage::from_config(&config);
    info!(
        redis_configured = redis.is_configured(),
        redis_connected = redis.is_connected(),
        pool_size = crate::core::redis::REDIS_POOL_SIZE,
        stream_maxlen = crate::core::redis::REDIS_STREAM_MAXLEN,
        fcm_configured = fcm_oauth.is_configured(),
        fcm_project = %fcm_oauth.resolved_project_id(),
        cloudinary_configured = cloudinary.is_configured(),
        "🧠 L1 AppCaches, RedisManager, FcmOAuthManager & CloudinaryStorage siap melayani."
    );
    let redis_pool = redis.active_pool();

    let chat_hub = Arc::new(ConnectionHub::new());
    let chat_bridge = RedisPubSubBridge::from_shared(
        Arc::clone(&chat_hub),
        redis_pool.clone(),
        redis.active_subscriber(),
    )
    .await;
    let chat_presence = Arc::new(PresenceService::new());
    let chat_messages = Arc::new(MessageService::with_member_cache(caches.chat_member_l1.clone()));
    let chat_moderation = Arc::new(ModerationGuard::new());

    let notif_redis_client = redis.active_client();
    let fcm_gateway = Arc::new(FcmV1Gateway::from_core(fcm_oauth.clone(), caches.clone()));
    let notif_router = Arc::new(OmnichannelRouter::new(
        pool.clone(),
        chat_bridge.clone(),
        chat_presence.clone(),
        fcm_gateway,
        notif_redis_client.clone(),
    ));
    let (notif_tx, notif_rx) = tokio::sync::mpsc::unbounded_channel();
    let notifications = Arc::new(NotificationProducerService::new(
        notif_redis_client.clone(),
        notif_tx,
    ));
    spawn_notification_workers(notif_redis_client, notif_rx, notif_router);

    let social_profile_cqrs = Arc::new(ProfileCqrsService::with_l1_cache(
        redis_pool.clone(),
        caches.profile_l1.clone(),
    ));
    let (club_cache, secret_join_service, taste_profiler) =
        core::init_module35_engines(&pool, redis_pool.clone(), &config);
    let social_graph_cache = Arc::new(
        crate::modules::social_graph::services::redis_graph_cache::RedisGraphCache::new(
            redis_pool,
        ),
    );
    let social_graph_notif = Arc::new(
        crate::modules::social_graph::services::follow_notif_dispatcher::FollowNotifDispatcher::new(
            chat_bridge.clone(),
            social_profile_cqrs.clone(),
        )
        .with_push_producer(notifications.clone()),
    );
    let social_graph_cmd = Arc::new(
        crate::modules::social_graph::services::graph_command_service::GraphCommandService::new(
            social_graph_cache.clone(),
            social_graph_notif,
        ),
    );
    let social_graph_query = Arc::new(
        crate::modules::social_graph::services::graph_query_service::GraphQueryService::new(
            social_graph_cache,
        ),
    );

    let rabbitmq = RabbitMqClient::connect_or_lazy(
        config
            .rabbitmq_url
            .clone()
            .map(secrecy::SecretString::from),
    )
    .await;

    let state = AppState {
        pool,
        config,
        jwt_keys,
        telemetry,
        rate_limiter,
        version_cache,
        chat_hub,
        chat_bridge,
        chat_presence,
        chat_messages,
        chat_moderation,
        social_profile_cqrs,
        social_graph_cmd,
        social_graph_query,
        notifications,
        rabbitmq: rabbitmq.clone(),
        redis,
        caches,
        fcm_oauth,
        cloudinary,
        club_cache,
        secret_join_service,
        taste_profiler,
    };

    spawn_rabbitmq_worker(rabbitmq, state.clone());
    let router = app::build_app(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!("🚀 Server berjalan di http://0.0.0.0:{}", port);
    info!("📊 Real-Time APM Dashboard: http://localhost:{}/dashboard", port);
    info!("📄 OpenAPI Specification:  http://localhost:{}/api/v1/openapi.json", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
