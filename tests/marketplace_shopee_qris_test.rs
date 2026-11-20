use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use coffe_labs_backend::{
    app,
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager, jwt::JwtManager,
        middleware::SecurityRateLimiter, migrations::marketplace_schema,
        rabbitmq::RabbitMqClient, redis::RedisManager, storage::CloudinaryStorage,
        telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        marketplace::services::qris_service::{
            build_dynamic_qris_payload, compute_emvco_crc16, enforce_qris_only_payment,
        },
        monitoring::MonitoringEngine,
        notifications::services::producer_service::NotificationProducerService,
        platform::models::AppVersionConfig,
        social_feed::services::profile_cqrs_service::ProfileCqrsService,
        social_graph::services::{
            graph_command_service::GraphCommandService,
            graph_query_service::GraphQueryService,
        },
    },
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn offline_test_state() -> AppState {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://postgres:postgres@127.0.0.1:59999/coffe_labs_test")
        .expect("lazy pool");
    let config = AppConfig {
        database_url: "postgres://postgres:postgres@127.0.0.1:59999/coffe_labs_test".to_string(),
        redis_url: String::new(),
        rabbitmq_url: None,
        google_application_credentials: None,
        fcm_project_id: "coffe-labs-test".to_string(),
        cloudinary_url: None,
        geoapify_api_key: None,
        jwt_secret: secrecy::SecretString::from(
            "test_secret_64_bytes_minimum_entropy_0123456789abcdef0123456789".to_string(),
        ),
        jwt_expiration_seconds: 3600,
        port: 3000,
        environment: "test".to_string(),
        rate_limit_per_second: 100,
        rate_limit_burst: 200,
    };
    let caches = AppCaches::new();
    let redis = RedisManager::disabled().expect("redis");
    let hub = Arc::new(ConnectionHub::new());
    let bridge = RedisPubSubBridge::from_shared(hub.clone(), None, None).await;
    let (club_cache, secret_join_service, taste_profiler) =
        coffe_labs_backend::core::init_module35_engines(&pool, None, &config);
    let (notif_tx, _) = tokio::sync::mpsc::unbounded_channel();
    let profile_cqrs = Arc::new(ProfileCqrsService::with_l1_cache(
        None,
        caches.profile_l1.clone(),
    ));
    let graph_cache = Arc::new(
        coffe_labs_backend::modules::social_graph::services::redis_graph_cache::RedisGraphCache::new(None),
    );
    let follow_notif = Arc::new(
        coffe_labs_backend::modules::social_graph::services::follow_notif_dispatcher::FollowNotifDispatcher::new(
            bridge.clone(),
            profile_cqrs.clone(),
        ),
    );
    AppState {
        pool,
        config,
        jwt: JwtManager::new("test_secret_64_bytes_minimum_entropy_0123456789abcdef0123456789", 3600),
        telemetry: TelemetryRegistry::new(),
        rate_limiter: SecurityRateLimiter::new(100, 200),
        version_cache: Arc::new(RwLock::new(AppVersionConfig::fallback_default())),
        chat_hub: hub,
        chat_bridge: bridge,
        chat_presence: Arc::new(PresenceService::new()),
        chat_messages: Arc::new(MessageService::with_member_cache(caches.chat_member_l1.clone())),
        chat_moderation: Arc::new(ModerationGuard::new()),
        social_profile_cqrs: profile_cqrs,
        social_graph_cmd: Arc::new(GraphCommandService::new(graph_cache.clone(), follow_notif)),
        social_graph_query: Arc::new(GraphQueryService::new(graph_cache)),
        notifications: Arc::new(NotificationProducerService::new(None, notif_tx)),
        rabbitmq: RabbitMqClient::disabled(),
        redis,
        caches,
        fcm_oauth: FcmOAuthManager::from_config(None, "coffe-labs-test"),
        cloudinary: CloudinaryStorage::unconfigured(),
        club_cache,
        secret_join_service,
        taste_profiler,
        monitoring: MonitoringEngine::init(None),
        live_brew_hub: Arc::new(
            coffe_labs_backend::modules::live_brew::services::live_telemetry_hub::LiveTelemetryHub::new(None),
        ),
        admin_control: coffe_labs_backend::modules::admin_control::RuntimeSnapshotStore::empty(None),
    }
}

#[test]
fn marketplace_schema_enforces_verified_bags_and_qris_orders() {
    let sql = marketplace_schema::statements().join("\n");
    assert!(sql.contains("VERIFIED_COFFEE_BAG"));
    assert!(sql.contains("marketplace_products"));
    assert!(sql.contains("marketplace_product_variants"));
    assert!(sql.contains("marketplace_cart_items"));
    assert!(sql.contains("qris_payload"));
    assert!(sql.contains("idx_mkt_verified_bestseller"));
}

#[test]
fn qris_dynamic_payload_builds_valid_emvco_crc16_and_blocks_non_qris_methods() {
    assert!(enforce_qris_only_payment("QRIS").is_ok());
    assert!(enforce_qris_only_payment("qris").is_ok());
    for disallowed in ["BCA VA", "Mandiri VA", "GoPay", "COD", ""] {
        assert!(
            enforce_qris_only_payment(disallowed).is_err(),
            "non-QRIS method {disallowed} must be rejected"
        );
    }

    let payload = build_dynamic_qris_payload("CL-2026-AB12CD", 155000);
    assert!(payload.starts_with("000201010212"));
    assert!(payload.contains("5303360"));
    assert!(payload.contains("5406155000"));
    assert!(payload.contains("5802ID"));
    assert!(payload.contains("COFFEE LABS ROASTERY"));
    assert!(payload.contains("CL-2026-AB12CD"));
    let without_crc = &payload[..payload.len() - 4];
    let crc = &payload[payload.len() - 4..];
    assert_eq!(crc, compute_emvco_crc16(without_crc));
}

#[tokio::test]
async fn checkout_endpoint_rejects_non_qris_payment_and_validates_vouchers() {
    let state = offline_test_state().await;
    let router = app::build_app(state);

    // Non-QRIS method must return 400 Bad Request even before DB insert
    let non_qris_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/marketplace/checkout")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "recipient_name": "Rusdi",
                        "phone_number": "08123456789",
                        "shipping_address": "Jl. Thamrin No 28 Jakarta",
                        "courier_name": "SiCepat BEST",
                        "shipping_fee": 20000,
                        "payment_method": "BCA VA",
                        "subtotal": 150000,
                        "items": []
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(non_qris_resp.status(), StatusCode::BAD_REQUEST);

    // Voucher COFFEEFIRST 15% validation works cleanly
    let voucher_resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/marketplace/vouchers/validate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "voucher_code": "COFFEEFIRST", "subtotal": 100000 }).to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(voucher_resp.status(), StatusCode::OK);
    let body = to_bytes(voucher_resp.into_body(), 16 * 1024).await.expect("bytes");
    let parsed: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(parsed["data"]["discount_amount"], json!(15000));
}
