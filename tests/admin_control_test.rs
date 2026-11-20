use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use coffe_labs_backend::{
    app::build_app,
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager, jwt::JwtManager,
        middleware::SecurityRateLimiter, rabbitmq::RabbitMqClient, redis::RedisManager,
        storage::CloudinaryStorage, telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        admin_control::{
            dto::{ControlSnapshotDto, FeatureControlDto},
            RuntimeSnapshotStore,
        },
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        live_brew::services::live_telemetry_hub::LiveTelemetryHub,
        monitoring::MonitoringEngine,
        notifications::services::producer_service::NotificationProducerService,
        platform::models::AppVersionConfig,
        social_feed::services::profile_cqrs_service::ProfileCqrsService,
        social_graph::services::{
            follow_notif_dispatcher::FollowNotifDispatcher,
            graph_command_service::GraphCommandService,
            graph_query_service::GraphQueryService,
            redis_graph_cache::RedisGraphCache,
        },
    },
};

async fn build_offline_admin_control_state() -> AppState {
    let config = AppConfig::from_env();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://postgres:postgres@127.0.0.1:5432/coffeelabs_test")
        .expect("lazy pool");
    let jwt = JwtManager::new(config.jwt_secret_str(), 3600);
    let redis = RedisManager::disabled().expect("disabled redis");
    let caches = AppCaches::new();
    let chat_hub = Arc::new(ConnectionHub::new());
    let chat_bridge = RedisPubSubBridge::from_shared(chat_hub.clone(), None, None).await;
    let social_profile_cqrs = Arc::new(ProfileCqrsService::with_l1_cache(
        None,
        caches.profile_l1.clone(),
    ));
    let (club_cache, secret_join_service, taste_profiler) =
        coffe_labs_backend::core::init_module35_engines(&pool, None, &config);
    let social_graph_cache = Arc::new(RedisGraphCache::new(None));
    let social_graph_notif = Arc::new(FollowNotifDispatcher::new(
        chat_bridge.clone(),
        social_profile_cqrs.clone(),
    ));
    let (notif_tx, _notif_rx) = tokio::sync::mpsc::unbounded_channel();

    AppState {
        pool,
        config: config.clone(),
        jwt,
        telemetry: TelemetryRegistry::new(),
        rate_limiter: SecurityRateLimiter::new(1000, 1000),
        version_cache: Arc::new(RwLock::new(AppVersionConfig::fallback_default())),
        chat_hub,
        chat_bridge,
        chat_presence: Arc::new(PresenceService::new()),
        chat_messages: Arc::new(MessageService::with_member_cache(
            caches.chat_member_l1.clone(),
        )),
        chat_moderation: Arc::new(ModerationGuard::new()),
        social_profile_cqrs,
        social_graph_cmd: Arc::new(GraphCommandService::new(
            social_graph_cache.clone(),
            social_graph_notif,
        )),
        social_graph_query: Arc::new(GraphQueryService::new(social_graph_cache)),
        notifications: Arc::new(NotificationProducerService::new(None, notif_tx)),
        rabbitmq: RabbitMqClient::disabled(),
        redis,
        caches,
        fcm_oauth: FcmOAuthManager::from_config(None, "test"),
        cloudinary: CloudinaryStorage::from_config(&config),
        club_cache,
        secret_join_service,
        taste_profiler,
        monitoring: MonitoringEngine::init(None),
        live_brew_hub: Arc::new(LiveTelemetryHub::new(None)),
        admin_control: RuntimeSnapshotStore::empty(None),
    }
}

#[tokio::test]
async fn web_control_panel_and_snapshot_etag_work() {
    let state = build_offline_admin_control_state().await;
    let app = build_app(state);

    let html_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/admin-control")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("res");
    assert_eq!(html_res.status(), StatusCode::OK);
    let body_bytes = to_bytes(html_res.into_body(), usize::MAX).await.expect("body");
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("Real-Time Admin Control Panel"));
    assert!(html.contains("Kontrol Afiliasi"));
    assert!(html.contains("/api/v1/admin/control/affiliate"));

    let snap_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/control-snapshot?since_version=1")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("res");
    assert_eq!(snap_res.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn feature_gate_middleware_blocks_non_admin_with_423_and_allows_admin_qa_bypass() {
    let state = build_offline_admin_control_state().await;
    state
        .admin_control
        .apply_and_broadcast_local(ControlSnapshotDto {
            config_version: 5,
            global_maintenance_mode: false,
            global_maintenance_message: None,
            connected_clients: 0,
            features: vec![FeatureControlDto {
                feature_key: "MARKETPLACE_KOPI".into(),
                display_name: "Marketplace Kopi".into(),
                group_section: "EKOSISTEM_KOMUNITAS".into(),
                route_prefix: "/marketplace".into(),
                api_path_prefixes: vec!["/api/v1/marketplace".into()],
                display_order: 5,
                is_locked: true,
                lock_status: "MAINTENANCE".into(),
                custom_badge_label: None,
                lock_title: "Marketplace Sedang Maintenance".into(),
                lock_message: "Sinkronisasi stok roastery 15 menit.".into(),
                estimated_unlock_at: None,
                allow_admin_bypass: true,
                updated_at: Utc::now(),
            }],
            plans: vec![],
            affiliate_config: Default::default(),
            updated_at: Utc::now(),
        });

    let user_jwt = state
        .jwt
        .generate_user_token(Uuid::new_v4(), "barista1", "user", "pro", "active")
        .expect("user jwt");
    let admin_jwt = state
        .jwt
        .generate_user_token(Uuid::new_v4(), "superadmin", "admin", "business", "active")
        .expect("admin jwt");

    let app = build_app(state);

    // 1. Non-admin hitting locked API prefix -> HTTP 423 Locked
    let locked_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/marketplace/products")
                .header(header::AUTHORIZATION, format!("Bearer {user_jwt}"))
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("res");
    assert_eq!(locked_res.status(), StatusCode::LOCKED);
    let body = to_bytes(locked_res.into_body(), usize::MAX).await.expect("b");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(json["code"], "FEATURE_LOCKED");
    assert_eq!(json["feature_key"], "MARKETPLACE_KOPI");
    assert_eq!(json["lock_status"], "MAINTENANCE");

    // 2. Admin hitting locked API prefix -> passes middleware (NOT 423!)
    let admin_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/marketplace/products")
                .header(header::AUTHORIZATION, format!("Bearer {admin_jwt}"))
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("res");
    assert_ne!(admin_res.status(), StatusCode::LOCKED);

    // 3. Non-admin trying to mutate Admin Control API -> HTTP 403 Forbidden
    let forbid_res = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/control/features/MARKETPLACE_KOPI")
                .header(header::AUTHORIZATION, format!("Bearer {user_jwt}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"is_locked":false,"lock_status":"UNLOCKED"}"#))
                .expect("req"),
        )
        .await
        .expect("res");
    assert_eq!(forbid_res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn snapshot_store_enforces_monotonic_version_and_supports_all_lock_statuses() {
    use coffe_labs_backend::modules::admin_control::dto::normalize_lock_status;

    assert_eq!(normalize_lock_status("MAINTENANCE", true).as_deref(), Some("MAINTENANCE"));
    assert_eq!(normalize_lock_status("coming_soon", true).as_deref(), Some("COMING_SOON"));
    assert_eq!(normalize_lock_status("UPDATED", true).as_deref(), Some("UPDATED"));
    assert_eq!(normalize_lock_status("MAINTENANCE", false).as_deref(), Some("UNLOCKED"));
    assert_eq!(normalize_lock_status("INVALID_STATUS", true), None);

    let store = RuntimeSnapshotStore::empty(None);
    store.apply_and_broadcast_local(ControlSnapshotDto {
        config_version: 5,
        global_maintenance_mode: false,
        global_maintenance_message: None,
        connected_clients: 0,
        features: vec![],
        plans: vec![],
        affiliate_config: Default::default(),
        updated_at: Utc::now(),
    });
    assert_eq!(store.load_snapshot().config_version, 5);

    // Stale v3 must be rejected (monotonic version guard)
    store.apply_and_broadcast_local(ControlSnapshotDto {
        config_version: 3,
        global_maintenance_mode: true,
        global_maintenance_message: Some("stale".into()),
        connected_clients: 0,
        features: vec![],
        plans: vec![],
        affiliate_config: Default::default(),
        updated_at: Utc::now(),
    });
    assert_eq!(store.load_snapshot().config_version, 5);
    assert!(!store.load_snapshot().global_maintenance_mode);

    use coffe_labs_backend::modules::affiliate::code_generator::is_tier_eligible_for_affiliate;
    assert!(!is_tier_eligible_for_affiliate("trial", "user", false));
    assert!(is_tier_eligible_for_affiliate("pro", "user", false));
    assert!(is_tier_eligible_for_affiliate("trial", "user", true));
    assert!(is_tier_eligible_for_affiliate("free", "user", true));
}
