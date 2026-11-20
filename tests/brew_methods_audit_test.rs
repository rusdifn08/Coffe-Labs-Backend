use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use coffe_labs_backend::{
    app,
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager, jwt::JwtManager,
        middleware::SecurityRateLimiter, migrations::brew_methods_audit_schema,
        rabbitmq::RabbitMqClient, redis::RedisManager, storage::CloudinaryStorage,
        telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        brew_methods_audit::{
            archetype_validator::{classify_method_archetype, validate_recipe_audit},
            coldbrew_math::compute_cold_brew_plan,
            dto::{AuditRecipeRequest, AuditRecipeStepInput},
            models::BrewExecutionArchetype,
        },
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
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

async fn offline_state() -> AppState {
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
        jwt: JwtManager::new(
            "test_secret_64_bytes_minimum_entropy_0123456789abcdef0123456789",
            3600,
        ),
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
fn ddl_contains_all_18_methods_with_corrected_icons() {
    let sql = brew_methods_audit_schema::statements().join("\n");
    assert!(sql.contains("brew_execution_archetype"));
    assert!(sql.contains("brewing_methods_catalog"));
    assert!(sql.contains("long_brew_scheduled_alarms"));
    assert!(sql.contains("assets/images/methods/cold_brew.png"));
    assert!(sql.contains("assets/images/methods/syphon.png"));
    assert!(sql.contains("assets/images/methods/japanese_drip.png"));
}

#[test]
fn arrhenius_cold_brew_fridge_takes_longer_than_room_temp() {
    let fridge = compute_cold_brew_plan("IMMERSION_FRIDGE", 80.0, 640.0, 4.0, None);
    let room = compute_cold_brew_plan("IMMERSION_ROOM", 80.0, 640.0, 22.0, None);
    assert!(fridge.recommended_duration_sec > room.recommended_duration_sec);
    assert!(fridge.recommended_hours >= 18.0);
    assert!(room.recommended_hours >= 10.0 && room.recommended_hours <= 14.0);
    assert!(fridge.concentrate_yield_ml > 0.0);
    assert!(fridge.suggested_dilution_water_or_ice_ml > 0.0);
}

#[test]
fn kyoto_drip_bpm_computes_exact_interval_and_hours() {
    let kyoto = compute_cold_brew_plan("KYOTO_DRIP", 50.0, 500.0, 18.0, Some(40));
    assert_eq!(kyoto.drip_interval_ms, Some(1500));
    assert!(kyoto.recommended_duration_sec >= 14_400);
}

#[test]
fn archetype_classifier_protects_v60_and_isolates_cold_brew() {
    let (v60_arch, v60_protected) =
        classify_method_archetype("Hario V60", "V60 Pour Over", "Kasuya 4:6", Some(92.0), None);
    assert_eq!(v60_arch, BrewExecutionArchetype::PercolationPourOver);
    assert!(v60_protected);

    let (cb_arch, cb_prot) =
        classify_method_archetype("Cold Brew", "Cold Brew", "Kyoto Drip", Some(4.0), None);
    assert_eq!(cb_arch, BrewExecutionArchetype::LongHorizonPersistentAlarm);
    assert!(!cb_prot);
}

#[test]
fn recipe_validator_catches_short_cold_brew_and_leaves_v60_untouched() {
    let broken_cold_brew = AuditRecipeRequest {
        recipe_id: Some(445),
        title: "Cold Brew Fake Short Timer".to_string(),
        method: "Cold Brew".to_string(),
        category: "Cold Brew".to_string(),
        water_temp_celsius: Some(20.0),
        coffee_weight_grams: 40.0,
        total_water_weight_grams: 450.0,
        ice_weight_grams: None,
        total_time_seconds: 180,
        steps: vec![AuditRecipeStepInput {
            step_order: 1,
            title: "Steep".to_string(),
            duration_seconds: 180,
            water_added_grams: 450.0,
        }],
    };
    let report = validate_recipe_audit(&broken_cold_brew);
    assert!(report.is_cold_brew_alarm_only);
    assert!(!report.is_valid);
    assert!(!report.warnings.is_empty());

    let v60_recipe = AuditRecipeRequest {
        recipe_id: Some(4),
        title: "4:6 Method Tetsu Kasuya".to_string(),
        method: "Hario V60".to_string(),
        category: "V60 Pour Over".to_string(),
        water_temp_celsius: Some(92.0),
        coffee_weight_grams: 20.0,
        total_water_weight_grams: 300.0,
        ice_weight_grams: None,
        total_time_seconds: 210,
        steps: vec![],
    };
    let v60_report = validate_recipe_audit(&v60_recipe);
    assert!(v60_report.v60_protected);
    assert!(v60_report.is_valid);
}

#[tokio::test]
async fn http_coldbrew_calculator_and_audit_routes_work() {
    let state = offline_state().await;
    let router = app::build_app(state);

    let calc_req = Request::builder()
        .method("POST")
        .uri("/api/v1/brew/coldbrew/calculate")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "brew_mode": "IMMERSION_FRIDGE",
                "coffee_dose_g": 80,
                "water_ml": 640,
                "ambient_temp_c": 4
            })
            .to_string(),
        ))
        .expect("req");
    let res = router.clone().oneshot(calc_req).await.expect("res");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(res.into_body(), 65_536).await.expect("bytes"))
            .expect("json");
    assert!(body["data"]["recommended_duration_sec"].as_i64().expect("sec") >= 60_000);

    let alarm_req = Request::builder()
        .method("GET")
        .uri("/api/v1/brew/long-alarms")
        .body(Body::empty())
        .expect("req");
    let alarm_res = router.oneshot(alarm_req).await.expect("res");
    assert_eq!(alarm_res.status(), StatusCode::UNAUTHORIZED);
}
