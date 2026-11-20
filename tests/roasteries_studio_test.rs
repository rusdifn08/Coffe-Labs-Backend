use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use coffe_labs_backend::{
    app,
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager, jwt::JwtManager,
        middleware::SecurityRateLimiter, migrations::roasteries_schema,
        rabbitmq::RabbitMqClient, redis::RedisManager, storage::CloudinaryStorage,
        telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        monitoring::MonitoringEngine,
        notifications::services::producer_service::NotificationProducerService,
        platform::models::AppVersionConfig,
        roasteries::dto::{compute_unit_price_labels, format_idr_price, slugify_roastery_name},
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
use uuid::Uuid;

async fn offline_state_with_cloudinary() -> AppState {
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
        cloudinary_url: Some("cloudinary://99887766:my_super_secret@irn72dwa".to_string()),
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
        cloudinary: CloudinaryStorage::from_url("cloudinary://99887766:my_super_secret@irn72dwa")
            .expect("cloudinary"),
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
fn roastery_migration_enforces_unique_owner_and_indexes() {
    let joined = roasteries_schema::statements().join("\n");
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS coffee_roasteries"));
    assert!(joined.contains("owner_user_id UUID NOT NULL UNIQUE"));
    assert!(joined.contains("CREATE TABLE IF NOT EXISTS roastery_coffee_lineups"));
    assert!(joined.contains("idx_roastery_published_cluster"));
    assert!(joined.contains("idx_roastery_lineup_filter"));
}

#[test]
fn unit_price_calculator_and_slugifier_match_catalog_format() {
    assert_eq!(format_idr_price(189_000), "Rp189.000");
    assert_eq!(
        compute_unit_price_labels(250, 189_000),
        (
            "Rp75.600".to_string(),
            "Rp151.200".to_string(),
            "Asli: Rp189.000 / 250g".to_string()
        )
    );
    assert_eq!(
        slugify_roastery_name("Senja Sangrai Labs! & Co.", "ab12cd34"),
        "senja-sangrai-labs-co-ab12cd34"
    );
}

#[tokio::test]
async fn business_roastery_endpoints_reject_non_business_and_sign_cloudinary_for_business() {
    let state = offline_state_with_cloudinary().await;
    let jwt = state.jwt.clone();
    let router = app::build_app(state);

    let trial_tok = jwt.generate_user_token(Uuid::new_v4(), "trialuser", "user", "Trial", "active").expect("tok");
    let pro_tok = jwt.generate_user_token(Uuid::new_v4(), "prouser", "user", "Pro", "active").expect("tok");
    let biz_id = Uuid::new_v4();
    let biz_tok = jwt.generate_user_token(biz_id, "bizowner", "user", "Business", "active").expect("tok");

    // 1. Unauthenticated -> 401
    let unauth_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/roasteries/me")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // 2. Trial & Pro -> 403 Forbidden
    for (tok, label) in [(trial_tok, "Trial"), (pro_tok, "Pro")] {
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/roasteries/media/upload-signature")
                    .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "asset_kind": "logo" }).to_string()))
                    .expect("req"),
            )
            .await
            .expect("resp");
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "tier {label} must be rejected");
    }

    // 3. Business -> 200 OK with scoped folder & signature
    let biz_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/roasteries/media/upload-signature")
                .header(header::AUTHORIZATION, format!("Bearer {biz_tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "asset_kind": "logo" }).to_string()))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(biz_resp.status(), StatusCode::OK);
    let body = to_bytes(biz_resp.into_body(), 16 * 1024).await.expect("bytes");
    let parsed: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(parsed["success"], json!(true));
    assert_eq!(
        parsed["data"]["folder"],
        json!(format!("coffeelabs/roasteries/{biz_id}/logo"))
    );
    assert_eq!(parsed["data"]["cloud_name"], json!("irn72dwa"));

    // 4. Invalid asset_kind -> 400 Bad Request
    let bad_resp = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/roasteries/media/upload-signature")
                .header(header::AUTHORIZATION, format!("Bearer {biz_tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "asset_kind": "invalid_kind" }).to_string()))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);
}
