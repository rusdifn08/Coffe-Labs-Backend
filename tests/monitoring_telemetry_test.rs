use std::{collections::HashMap, io::Write, sync::Arc};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use chrono::{Duration, Utc};
use coffe_labs_backend::{
    app,
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager, jwt::JwtManager,
        middleware::SecurityRateLimiter, migrations::monitoring_schema, rabbitmq::RabbitMqClient,
        redis::RedisManager, storage::CloudinaryStorage, telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        monitoring::{
            dto::{MAX_IDLE_BREW_OR_AOD_SEC, MAX_IDLE_REGULAR_SEC},
            models::LiveAccountState,
            services::{
                batch_flusher_worker::{MonitoringIngestQueue, MPSC_QUEUE_CAPACITY},
                bitmap_analytics_service::BitmapAnalyticsService,
                dossier_aggregator::{
                    build_feature_breakdown, classify_churn_risk, compute_health_score,
                    normalize_feature_bucket, sanitize_event_duration,
                },
                live_presence_tracker::LivePresenceTracker,
            },
            MonitoringEngine,
        },
        notifications::services::producer_service::NotificationProducerService,
        platform::models::AppVersionConfig,
        social_feed::services::profile_cqrs_service::ProfileCqrsService,
        social_graph::services::{
            graph_command_service::GraphCommandService,
            graph_query_service::GraphQueryService,
        },
    },
};
use flate2::{write::GzEncoder, Compression};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

async fn build_offline_test_state() -> AppState {
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
    let redis = RedisManager::disabled().expect("disabled redis");
    let hub = Arc::new(ConnectionHub::new());
    let bridge = RedisPubSubBridge::from_shared(hub.clone(), None, None).await;
    let (club_cache, secret_join_service, taste_profiler) =
        coffe_labs_backend::core::init_module35_engines(&pool, None, &config);
    let (notif_tx, _notif_rx) = tokio::sync::mpsc::unbounded_channel();
    let notifications = Arc::new(NotificationProducerService::new(None, notif_tx));
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
        notifications,
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
fn migration_defines_partitioned_telemetry_and_bitmap_seq() {
    let sql_joined = monitoring_schema::statements().join("\n");
    for required in [
        "user_bitmap_seq",
        "bitmap_seq_id",
        "account_monitoring_dossiers",
        "account_daily_telemetry_rollups",
        "account_brew_experiment_logs",
        "telemetry_daily_bitmaps",
        "telemetry_raw_events",
        "PARTITION BY RANGE (recorded_at)",
        "telemetry_raw_events_default",
    ] {
        assert!(sql_joined.contains(required), "Missing SQL construct: {required}");
    }
}

#[test]
fn idle_guard_clamps_regular_and_separates_aod_screen_time() {
    let regular = sanitize_event_duration("SCREEN_VIEW", "social_coffee", 9999, false);
    assert_eq!(regular.active_screen_sec, MAX_IDLE_REGULAR_SEC);
    assert_eq!(regular.aod_kiosk_sec, 0);

    let brew = sanitize_event_duration("SCREEN_VIEW", "brew_timer", 720, false);
    assert_eq!(brew.active_screen_sec, 720);
    assert_eq!(brew.aod_kiosk_sec, 0);

    let capped_brew = sanitize_event_duration("BREW_COMPLETE", "brew_timer", 2000, false);
    assert_eq!(capped_brew.active_screen_sec, MAX_IDLE_BREW_OR_AOD_SEC);

    let aod = sanitize_event_duration("SCREEN_VIEW", "aod_kiosk", 650, true);
    assert_eq!(aod.active_screen_sec, 0);
    assert_eq!(aod.aod_kiosk_sec, 650);

    let neg = sanitize_event_duration("SCREEN_VIEW", "general", -45, false);
    assert_eq!(neg.active_screen_sec, 0);
    assert_eq!(neg.aod_kiosk_sec, 0);
}

#[test]
fn health_score_churn_and_feature_breakdown_math_is_accurate() {
    let healthy = compute_health_score(0, 7, 6, 4);
    assert!(healthy >= 85);
    assert_eq!(classify_churn_risk(Some(0), healthy), "LOW");

    let stale = compute_health_score(16, 0, 0, 1);
    assert_eq!(classify_churn_risk(Some(16), stale), "DORMANT");

    let high_risk = compute_health_score(8, 1, 0, 1);
    assert_eq!(classify_churn_risk(Some(8), high_risk), "HIGH");

    let breakdown = build_feature_breakdown(&json!({
        "brew_timer": 600,
        "social_coffee": 300,
        "ai_scanner": 100
    }));
    assert_eq!(breakdown.len(), 3);
    assert_eq!(breakdown[0].feature, "brew_timer");
    assert_eq!(breakdown[0].share_percent, 60.0);
    assert_eq!(normalize_feature_bucket("clubs"), "coffee_club");
}

#[test]
fn roaring_bitmap_dau_wau_mau_cohort_and_adoption_work_correctly() {
    let svc = BitmapAnalyticsService::new();
    let today = Utc::now().date_naive();
    let day1_ago = today - Duration::days(1);
    let day3_ago = today - Duration::days(3);

    // Day -3: users 1, 2, 3, 4 active (cohort size = 4)
    for uid in 1..=4 {
        svc.record_user_activity(day3_ago, uid, "brew_timer", "pro", false);
    }
    // Day -2: users 1, 2 active (D1 = 2/4 = 50%)
    svc.record_user_activity(day3_ago + Duration::days(1), 1, "brew_timer", "pro", false);
    svc.record_user_activity(day3_ago + Duration::days(1), 2, "social_coffee", "trial", false);

    // Day -1 & Today: users 1, 5 active today
    svc.record_user_activity(day1_ago, 1, "brew_timer", "pro", false);
    svc.record_user_activity(today, 1, "brew_timer", "pro", false);
    svc.record_user_activity(today, 5, "coffee_club", "business", false);

    let (dau, wau, mau, stickiness) = svc.compute_dau_wau_mau(today);
    assert_eq!(dau, 2);
    assert_eq!(wau, 5);
    assert_eq!(mau, 5);
    assert_eq!(stickiness, 40.0);

    let mut mins = HashMap::new();
    mins.insert("brew_timer".to_string(), 42);
    let adoption = svc.compute_feature_adoption_7d(today, &mins);
    let brew_stat = adoption
        .iter()
        .find(|f| f.feature_key == "brew_timer")
        .expect("brew_timer adoption");
    assert_eq!(brew_stat.active_users_7d, 4);
    assert_eq!(brew_stat.adoption_percent, 80.0);
    assert_eq!(brew_stat.total_minutes_7d, 42);

    let cohorts = svc.compute_retention_cohorts(today);
    let c3 = cohorts
        .iter()
        .find(|c| c.cohort_date == day3_ago)
        .expect("cohort 3 days ago");
    assert_eq!(c3.cohort_size, 4);
    assert_eq!(c3.d1_pct, Some(50.0));
    assert_eq!(c3.d3_pct, Some(25.0));

    // Dirty snapshot roundtrip serialization + RAM eviction
    let dirty = svc.drain_dirty_snapshots();
    assert!(!dirty.is_empty());
    let reloaded = BitmapAnalyticsService::new();
    for (d, k, bytes, _) in dirty {
        reloaded.load_bitmap_bytes(d, k, &bytes);
    }
    assert_eq!(reloaded.compute_dau_wau_mau(today).0, 2);

    svc.record_user_activity(today - Duration::days(60), 99, "general", "trial", false);
    let before = svc.entry_count();
    svc.evict_old_days(today);
    assert!(svc.entry_count() < before);
}

#[tokio::test]
async fn mpsc_queue_bounds_backpressure_and_live_tracker_sweeps_stale() {
    let (queue, mut rx, _) = MonitoringIngestQueue::new();
    let sample_ev = coffe_labs_backend::modules::monitoring::repositories::bulk_ingest_repo::PendingRawEvent {
        event_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        event_type: "SCREEN_VIEW".to_string(),
        feature_bucket: "brew_timer".to_string(),
        route_name: "/brew/timer".to_string(),
        active_screen_sec: 12,
        aod_kiosk_sec: 0,
        metadata_json: json!({}),
        recorded_at: Utc::now(),
        platform: "android".to_string(),
        app_version: None,
        device_model: None,
    };

    for _ in 0..MPSC_QUEUE_CAPACITY {
        assert!(queue.enqueue(vec![sample_ev.clone()]));
    }
    assert!(!queue.enqueue(vec![sample_ev]));
    assert_eq!(queue.dropped_total(), 1);
    let _ = rx.try_recv();

    let tracker = LivePresenceTracker::new(None);
    let fresh_uid = Uuid::new_v4();
    let stale_uid = Uuid::new_v4();
    let now_epoch = Utc::now().timestamp();
    tracker
        .upsert_live_state(LiveAccountState {
            user_id: fresh_uid,
            current_route: "/social".to_string(),
            feature_bucket: "social_coffee".to_string(),
            tier: "Pro".to_string(),
            platform: "android".to_string(),
            app_version: "2.3.0".to_string(),
            is_brewing_now: false,
            last_seen_epoch: now_epoch,
        })
        .await;
    tracker
        .upsert_live_state(LiveAccountState {
            user_id: stale_uid,
            current_route: "/".to_string(),
            feature_bucket: "general".to_string(),
            tier: "Trial".to_string(),
            platform: "android".to_string(),
            app_version: "2.3.0".to_string(),
            is_brewing_now: false,
            last_seen_epoch: now_epoch - 250,
        })
        .await;

    assert_eq!(tracker.count_live_now(), 1);
    let expired = tracker.sweep_expired();
    assert_eq!(expired, vec![stale_uid]);
}

#[tokio::test]
async fn router_enforces_rbac_accepts_gzip_ingest_and_serves_dashboard() {
    let state = build_offline_test_state().await;
    let jwt = state.jwt.clone();
    let monitoring = state.monitoring.clone();
    let router = app::build_app(state);

    // 1. /dashboard serves upgraded Mobile-Palette Account & Feature Monitoring UI + Nav Buttons
    let dash_res = router
        .clone()
        .oneshot(Request::builder().uri("/dashboard").body(Body::empty()).expect("req"))
        .await
        .expect("resp");
    assert_eq!(dash_res.status(), StatusCode::OK);
    let dash_bytes = to_bytes(dash_res.into_body(), 2 * 1024 * 1024).await.expect("body");
    let dash_html = String::from_utf8_lossy(&dash_bytes);
    assert!(dash_html.contains("Account &amp; Feature Monitoring") || dash_html.contains("Account & Feature Monitoring"));
    assert!(dash_html.contains("mon-dossier-modal"));
    assert!(dash_html.contains("#F9F6F0"));
    assert!(dash_html.contains("#6B4423"));
    assert!(dash_html.contains("nav-badge-online"));
    assert!(dash_html.contains("Monitor API Request"));

    let mon_route_res = router
        .clone()
        .oneshot(Request::builder().uri("/dashboard/monitoring").body(Body::empty()).expect("req"))
        .await
        .expect("resp");
    assert_eq!(mon_route_res.status(), StatusCode::OK);
    let mon_bytes = to_bytes(mon_route_res.into_body(), 2 * 1024 * 1024).await.expect("body");
    assert!(String::from_utf8_lossy(&mon_bytes).contains("data-default-tab=\"monitoring\""));

    for (uri, tab) in [("/dashboard/admin", "admin"), ("/dashboard/services", "services")] {
        let r = router
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).expect("req"))
            .await
            .expect("resp");
        assert_eq!(r.status(), StatusCode::OK);
        let b = to_bytes(r.into_body(), 2 * 1024 * 1024).await.expect("body");
        assert!(String::from_utf8_lossy(&b).contains(&format!("data-default-tab=\"{tab}\"")));
    }

    // 2. /api/v1/monitoring/overview enforces Zero-Trust JWT (401 unauthenticated, 403 non-admin Trial)
    let unauth_res = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/monitoring/overview")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);

    let trial_uid = Uuid::new_v4();
    let trial_tok = jwt
        .generate_user_token(trial_uid, "trialbarista", "user", "Trial", "approved")
        .expect("trial token");
    let forbidden_res = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/monitoring/accounts")
                .header(header::AUTHORIZATION, format!("Bearer {trial_tok}"))
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(forbidden_res.status(), StatusCode::FORBIDDEN);

    // 3. POST /api/v1/monitoring/ingest-batch supports gzip decompression cleanly
    let payload = json!({
        "platform": "android",
        "app_version": "2.3.0",
        "active_route": "/brew/timer",
        "active_feature": "brew_timer",
        "is_brewing": true,
        "events": [{
            "event_id": Uuid::new_v4(),
            "session_id": Uuid::new_v4(),
            "event_type": "BREW_COMPLETE",
            "feature_bucket": "brew_timer",
            "route_name": "/brew/timer",
            "duration_sec": 195,
            "is_aod_mode": false,
            "metadata_json": { "recipe_title": "Tetsu Kasuya 4:6 V60", "brew_method": "V60", "completed": true },
            "recorded_at": Utc::now().to_rfc3339()
        }]
    });
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(payload.to_string().as_bytes())
        .expect("gzip write");
    let gz_body = encoder.finish().expect("gzip finish");

    let ingest_res = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/monitoring/ingest-batch")
                .header(header::AUTHORIZATION, format!("Bearer {trial_tok}"))
                .header(header::CONTENT_ENCODING, "gzip")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(gz_body))
                .expect("ingest req"),
        )
        .await
        .expect("ingest resp");
    assert_eq!(ingest_res.status(), StatusCode::ACCEPTED);
    let body_bytes = to_bytes(ingest_res.into_body(), 64 * 1024).await.expect("bytes");
    let parsed: Value = serde_json::from_slice(&body_bytes).expect("json");
    assert_eq!(parsed["data"]["accepted_events"], 1);
    assert_eq!(monitoring.live_presence.count_live_now(), 1);
}
