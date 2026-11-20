use std::sync::Arc;

use axum::{body::Body, http::{Request, StatusCode}};
use chrono::TimeZone;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use coffe_labs_backend::{
    app,
    core::{
        cache::AppCaches, config::AppConfig, database::init_db, fcm_oauth::FcmOAuthManager,
        jwt::JwtManager, middleware::SecurityRateLimiter, rabbitmq::RabbitMqClient,
        redis::RedisManager, storage::CloudinaryStorage, telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        beans_radar::{
            models::{CompactNodeDbRow, CompactSpatialNode},
            opening_hours_eval::evaluate_open_status_at,
            services::rtree_memory_engine::{RTreeMemoryEngine, RTreeQueryFilter},
            spatial_math::{
                chord_dist_sq_for_meters, compute_bayesian_score, compute_smart_discovery_score,
                estimate_eta_minutes, format_distance_label, haversine_meters, to_unit_sphere,
            },
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

async fn offline_test_state() -> AppState {
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
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&config.database_url)
        .expect("lazy pool");
    let redis = RedisManager::disabled().expect("disabled redis");
    let caches = AppCaches::new();
    let jwt = JwtManager::new(
        "test_secret_64_bytes_minimum_entropy_0123456789abcdef0123456789",
        3600,
    );
    let telemetry = TelemetryRegistry::new();
    let rate_limiter = SecurityRateLimiter::new(100, 200);
    let version_cache = Arc::new(RwLock::new(AppVersionConfig::fallback_default()));
    let chat_hub = Arc::new(ConnectionHub::new());
    let chat_bridge = RedisPubSubBridge::from_shared(Arc::clone(&chat_hub), None, None).await;
    let chat_presence = Arc::new(PresenceService::new());
    let chat_messages = Arc::new(MessageService::with_member_cache(caches.chat_member_l1.clone()));
    let chat_moderation = Arc::new(ModerationGuard::new());
    let social_profile_cqrs = Arc::new(ProfileCqrsService::with_l1_cache(
        None,
        caches.profile_l1.clone(),
    ));
    let (notif_tx, _) = tokio::sync::mpsc::unbounded_channel();
    let notifications = Arc::new(NotificationProducerService::new(None, notif_tx));
    let (club_cache, secret_join_service, taste_profiler) =
        coffe_labs_backend::core::init_module35_engines(&pool, None, &config);
    let graph_cache = Arc::new(RedisGraphCache::new(None));
    let graph_notif = Arc::new(FollowNotifDispatcher::new(
        chat_bridge.clone(),
        social_profile_cqrs.clone(),
    ));
    let social_graph_cmd = Arc::new(GraphCommandService::new(graph_cache.clone(), graph_notif));
    let social_graph_query = Arc::new(GraphQueryService::new(graph_cache));
    let monitoring = MonitoringEngine::init(None);
    let live_brew_hub = Arc::new(LiveTelemetryHub::new(None));
    let rabbitmq = RabbitMqClient::disabled();

    AppState {
        pool,
        config,
        jwt,
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
        rabbitmq,
        redis,
        caches,
        fcm_oauth: FcmOAuthManager::from_config(None, "coffe-labs-test"),
        cloudinary: CloudinaryStorage::unconfigured(),
        club_cache,
        secret_join_service,
        taste_profiler,
        monitoring,
        live_brew_hub,
        admin_control: coffe_labs_backend::modules::admin_control::RuntimeSnapshotStore::empty(None),
    }
}

#[test]
fn unit_sphere_rtree_matches_exact_haversine_sub_millisecond() {
    let places = vec![
        ("Monas Roast", -6.1754, 106.8272),
        ("Senopati Slowbar", -6.2312, 106.8112),
        ("Dago Roastery", -6.9015, 107.6128),
        ("Kintamani Highland", -8.2412, 115.3248),
    ];
    let nodes: Vec<CompactSpatialNode> = places
        .iter()
        .map(|(_, lat, lng)| CompactSpatialNode::from_compact_row(&CompactNodeDbRow {
            id: Uuid::new_v4(),
            latitude: *lat,
            longitude: *lng,
            category: "ROASTERY".into(),
            rating: 4.9,
            bayesian_score: compute_bayesian_score(4.9, 120),
            sells_roast_beans: true,
            is_verified_partner: true,
        }))
        .collect();

    let tree = rstar::RTree::bulk_load(nodes);
    let user_lat = -6.1944;
    let user_lng = 106.8229;
    let query_pt = to_unit_sphere(user_lat, user_lng);
    let max_sq = chord_dist_sq_for_meters(10_000.0);

    let t0 = std::time::Instant::now();
    let within_10km: Vec<_> = tree.locate_within_distance(query_pt, max_sq).collect();
    let elapsed_us = t0.elapsed().as_micros();

    assert_eq!(within_10km.len(), 2);
    assert!(elapsed_us < 1000, "R*-Tree query {} us", elapsed_us);

    let d_bandung = haversine_meters(user_lat, user_lng, -6.9015, 107.6128);
    assert!(d_bandung > 110_000.0 && d_bandung < 130_000.0);
    assert_eq!(format_distance_label(340.0), "340 m");
    assert_eq!(format_distance_label(1840.0), "1.8 km");
    assert_eq!(estimate_eta_minutes(340.0), (Some(6), 2));
    assert!(compute_smart_discovery_score(4.8, 300.0, true, true) > 0.85);
}

#[test]
fn multi_timezone_opening_hours_evaluates_wib_wita_wit_and_overnight() {
    let hours = serde_json::json!({
        "thu": [["08:00", "22:00"]],
        "fri": [["08:00", "02:00"]]
    });
    let utc_1430 = chrono::Utc
        .with_ymd_and_hms(2026, 11, 19, 14, 30, 0)
        .single()
        .expect("valid dt");
    let wib_status = evaluate_open_status_at(&hours, false, "Asia/Jakarta", utc_1430);
    assert!(wib_status.is_open_now);
    assert!(wib_status.is_closing_soon);
    assert!(wib_status.status_label_id.contains("WIB"));

    let wita_status = evaluate_open_status_at(&hours, false, "Asia/Makassar", utc_1430);
    assert!(!wita_status.is_open_now);
    assert!(wita_status.status_label_id.contains("WITA"));

    let wit_24 = evaluate_open_status_at(&hours, true, "Asia/Jayapura", utc_1430);
    assert!(wit_24.is_open_now);
    assert!(wit_24.status_label_id.contains("WIT"));
}

#[tokio::test]
async fn router_validates_zero_coords_out_of_bounds_radius_and_auth() {
    let state = offline_test_state().await;
    let router = app::build_app(state);

    let res_zero = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/beans-radar/nearby?lat=0.0&lng=0.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res_zero.status().is_client_error());

    let res_radius = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/beans-radar/nearby?lat=-6.9175&lng=107.6191&radius_km=999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res_radius.status().is_client_error());

    let fav_uri = format!("/api/v1/beans-radar/places/{}/favorite", Uuid::new_v4());
    let res_auth = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&fav_uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_auth.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_neondb_rtree_loads_over_1000_venues_and_queries_5_cities_sub_5ms() {
    let Ok(env_text) = std::fs::read_to_string(".env") else {
        return;
    };
    let Some(db_url) = env_text
        .lines()
        .find_map(|l| l.trim().strip_prefix("DATABASE_URL="))
        .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
    else {
        return;
    };

    let pool = init_db(&db_url).await.expect("connect NeonDB");
    let engine = RTreeMemoryEngine::empty();
    let loaded = engine.reload_from_db(&pool).await;
    assert!(loaded >= 1000, "Harus >= 1000 titik di R*-Tree, aktual {}", loaded);

    let filter = RTreeQueryFilter {
        category_code: None,
        min_rating_x10: 0,
        sells_beans_only: false,
    };
    let test_hubs = [
        ("Bandung", -6.9175, 107.6191),
        ("Jakarta", -6.1944, 106.8229),
        ("Yogyakarta", -7.7956, 110.3695),
        ("Surabaya", -7.2575, 112.7521),
        ("Bali", -8.6500, 115.2167),
    ];

    for (city, lat, lng) in test_hubs {
        let t0 = std::time::Instant::now();
        let candidates = engine.locate_within_radius(lat, lng, 15.0, &filter, 50);
        let elapsed_us = t0.elapsed().as_micros();
        assert!(
            candidates.len() >= 5,
            "{} harus memiliki >= 5 tempat dalam 15 km, aktual {}",
            city,
            candidates.len()
        );
        assert!(
            elapsed_us < 5000,
            "{} R*-Tree query {} us melebihi 5 ms!",
            city,
            elapsed_us
        );
        for pair in candidates.windows(2) {
            assert!(pair[0].distance_meters <= pair[1].distance_meters);
        }
        println!(
            "[RUST R*-TREE] {} -> {} matches (loaded={}) in {} us (top1={:.1}m)",
            city,
            candidates.len(),
            loaded,
            elapsed_us,
            candidates[0].distance_meters
        );
    }
}
