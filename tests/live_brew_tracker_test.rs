use std::{collections::HashSet, sync::Arc};

use axum::{body::Body, http::{Request, StatusCode}};
use coffe_labs_backend::{
    app::build_app,
    core::{
        cache::AppCaches, config::AppConfig, jwt::JwtManager, middleware::SecurityRateLimiter,
        migrations::live_brew_schema, rabbitmq::RabbitMqClient, redis::RedisManager,
        storage::CloudinaryStorage, telemetry::TelemetryRegistry, fcm_oauth::FcmOAuthManager,
        AppState,
    },
    modules::{
        chat::services::{
            connection_hub::ConnectionHub, message_service::MessageService,
            moderation_guard::ModerationGuard, presence_service::PresenceService,
            redis_pubsub_bridge::RedisPubSubBridge,
        },
        live_brew::{
            address_cipher::{
                encode_dynamic_session_code, encode_static_table_code, landing_livebrew_url,
                verify_and_classify_code, verify_luhn_13, AddressCodeKind,
            },
            dto::{CatchupCurvePoint, WebRtcSignalPayload},
            mirror_dto::LiveSessionMirrorSnapshot,
            mirror_models::slugify_livebrew_segment,
            mirror_protocol::{
                BrewerMirrorCommand, LiveBrewMirrorPhase, LiveTimerStatus, MirrorDownstreamFrame,
            },
            protocol::{LiveBrewStage, ViewerDownstreamEvent},
            services::{
                live_telemetry_hub::LiveTelemetryHub,
                mirror_state_hub::MirrorStateHub,
                mirror_state_machine::apply_mirror_command,
            },
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
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

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
        live_brew_hub: Arc::new(LiveTelemetryHub::new(None)),
        admin_control: coffe_labs_backend::modules::admin_control::RuntimeSnapshotStore::empty(None),
    }
}

#[test]
fn live_brew_migration_contains_all_tables_and_enum() {
    let joined = live_brew_schema::statements().join("\n");
    for sym in [
        "business_brew_tables",
        "static_address_code",
        "live_brew_stage",
        "live_brew_sessions",
        "dynamic_address_code",
        "recorded_telemetry_curve",
        "webrtc_whep_endpoint",
    ] {
        assert!(joined.contains(sym), "Missing {sym} in live_brew DDL");
    }
}

#[test]
fn feistel_cipher_and_luhn_are_bijective_and_reject_tampering() {
    let secret = "coffe-labs-feistel-test-secret-2026";
    let mut seen_dyn = HashSet::new();

    for seq in 1..=250_u64 {
        let dyn_code = encode_dynamic_session_code(seq, secret);
        assert_eq!(dyn_code.len(), 13);
        assert!(dyn_code.starts_with('3'));
        assert!(verify_luhn_13(&dyn_code));
        assert!(seen_dyn.insert(dyn_code.clone()), "Duplicate dynamic code at {seq}");

        let classified = verify_and_classify_code(&dyn_code, secret).expect("valid dyn code");
        assert_eq!(classified, AddressCodeKind::DynamicSession { seq_hint: seq });

        // Flip 1 digit -> Luhn rejects cleanly
        let mut tampered_bytes = dyn_code.clone().into_bytes();
        tampered_bytes[10] = if tampered_bytes[10] == b'9' { b'0' } else { tampered_bytes[10] + 1 };
        let tampered = String::from_utf8(tampered_bytes).expect("utf8");
        assert!(verify_and_classify_code(&tampered, secret).is_err());
    }

    let tbl_code = encode_static_table_code(4, secret);
    assert_eq!(tbl_code.len(), 13);
    assert!(tbl_code.starts_with('7'));
    assert_eq!(
        verify_and_classify_code(&tbl_code, secret).expect("valid table code"),
        AddressCodeKind::StaticTable { seq_hint: 4 }
    );
    assert_eq!(landing_livebrew_url(&tbl_code), format!("https://coffeelabs.app/livebrew/{tbl_code}"));
}

#[tokio::test]
async fn live_telemetry_hub_broadcasts_downsamples_catchup_and_stores_whip() {
    let hub = LiveTelemetryHub::new(None);
    let sid = Uuid::now_v7();
    let mut viewer_rx = hub.subscribe_session(sid);

    for tick in 0..=10 {
        let t_ms = tick * 100;
        let pt = CatchupCurvePoint {
            t_ms,
            w_g: tick as f32 * 5.0,
            flow: 5.0,
            temp_c: 92.0,
        };
        hub.append_curve_point(sid, pt.clone());
        hub.broadcast_to_session(sid, Arc::new(ViewerDownstreamEvent::TelemetryTick(pt)));
    }

    let catchup = hub.get_catchup_curve(sid).await;
    assert_eq!(catchup.len(), 3, "Expected 0ms, 500ms, 1000ms downsampled points");

    let first_ev = viewer_rx.recv().await.expect("first event");
    assert!(matches!(*first_ev, ViewerDownstreamEvent::TelemetryTick(_)));

    hub.store_whip_offer(sid, "v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\n".to_string());
    hub.push_ice_candidate(
        sid,
        &WebRtcSignalPayload {
            sdp: None,
            candidate: Some("candidate:1 1 UDP 2122260223 192.168.1.2 54400 typ host".to_string()),
            sdp_mid: Some("0".to_string()),
            sdp_mline_index: Some(0),
        },
    );
    assert!(hub.get_whip_offer(sid).is_some());
    assert_eq!(hub.take_ice_candidates(sid).len(), 1);
    assert_eq!(LiveBrewStage::BloomingDegassing.as_db_str(), "BLOOMING_DEGASSING");
}

#[tokio::test]
async fn router_validates_luhn_in_ram_and_protects_barista_endpoints() {
    let app = build_app(offline_state().await);

    let bad_resolve = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/livebrew/resolve/3000000000009")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(bad_resolve.status(), StatusCode::BAD_REQUEST);

    let unauth_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/livebrew/sessions")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"recipe_title":"V60 Test"}"#))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(unauth_create.status(), StatusCode::UNAUTHORIZED);

    // v2.0 NTP calibration endpoint returns 200 OK
    let ntp_res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/livebrew/ntp?client_send_ms=1760000000000")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(ntp_res.status(), StatusCode::OK);

    // v2.0 Business Branding endpoint rejects non-authenticated requests
    let unauth_branding = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/livebrew/business/branding")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"cafe_slug":"senja-bar","cafe_display_name":"Senja Bar"}"#))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(unauth_branding.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn live_brew_v2_8_phase_mirror_state_machine_and_ntp_anchor() {
    let ddl = live_brew_schema::statements().join("\n");
    for sym in [
        "live_brew_mirror_phase",
        "business_livebrew_branding",
        "business_cafe_seats",
        "live_brew_real_sessions",
        "timer_anchor_server_ms",
        "state_version",
    ] {
        assert!(ddl.contains(sym), "Missing v2 DDL symbol {sym}");
    }

    assert_eq!(slugify_livebrew_segment("Kopi Senja BDG!", 64), "kopi-senja-bdg");
    assert_eq!(slugify_livebrew_segment("Meja 04", 48), "meja-04");

    let sid = Uuid::now_v7();
    let mut snap = LiveSessionMirrorSnapshot {
        session_id: sid,
        share_code: "3100000000018".into(),
        share_url: "https://coffeelabs.com/livebrew/senja-roastery/meja-04".into(),
        deep_link_url: "coffeelabs://livebrew/senja-roastery/meja-04".into(),
        is_business_session: true,
        store_id: Some(Uuid::now_v7()),
        seat_id: Some(Uuid::now_v7()),
        pos_order_id: Some(Uuid::now_v7()),
        pos_order_number: Some("ORD-042".into()),
        pos_order_status: Some("QUEUED".into()),
        brewer_display_name: "Barista Raka".into(),
        brewer_avatar_url: None,
        customer_name: "Meja 04".into(),
        phase: LiveBrewMirrorPhase::PosOrderQueued,
        phase_label_id: LiveBrewMirrorPhase::PosOrderQueued.label_id().into(),
        state_version: 1,
        selected_method_id: None,
        selected_method_name: None,
        selected_recipe_id: None,
        selected_recipe_title: None,
        recipe_snapshot_json: serde_json::json!({}),
        selected_bean_id: None,
        bean_snapshot_json: serde_json::json!({}),
        timer_status: LiveTimerStatus::Idle,
        timer_accumulated_ms: 0,
        timer_anchor_server_ms: None,
        current_step_index: 0,
        current_weight_g: 0.0,
        current_flow_rate_ml_s: 0.0,
        current_temp_c: 91.0,
        telemetry_curve_json: serde_json::json!([]),
        cheers_count: 0,
        active_viewers_count: 0,
        server_epoch_ms: 1_000_000,
    };

    // 1. Brewer selects recipe live
    assert!(apply_mirror_command(
        &mut snap,
        &BrewerMirrorCommand::SelectRecipePreview {
            method_id: "v60".into(),
            method_name: "V60 Pour Over".into(),
            recipe_id: None,
            recipe_title: "Tetsu Kasuya 4:6".into(),
            recipe_snapshot_json: serde_json::json!({"coffeeWeightGrams": 15, "totalWaterWeightGrams": 225}),
        },
        1_001_000,
    ));
    assert_eq!(snap.phase, LiveBrewMirrorPhase::BrewerSelectingRecipe);
    assert_eq!(snap.state_version, 2);

    // 2. Brewer selects coffee bean live
    assert!(apply_mirror_command(
        &mut snap,
        &BrewerMirrorCommand::SelectBeanPreview {
            bean_id: None,
            bean_snapshot_json: serde_json::json!({"name": "Fugol Halu Banana", "process": "Wine Natural"}),
        },
        1_002_000,
    ));
    assert_eq!(snap.phase, LiveBrewMirrorPhase::BrewerSelectingBean);
    assert_eq!(snap.state_version, 3);

    // 3. Confirm Ready -> Start NTP-Anchored Timer at t = 1_005_000 ms
    assert!(apply_mirror_command(&mut snap, &BrewerMirrorCommand::ConfirmReadyToBrew, 1_003_000));
    assert!(apply_mirror_command(&mut snap, &BrewerMirrorCommand::TimerStartOrResume, 1_005_000));
    assert_eq!(snap.phase, LiveBrewMirrorPhase::BrewingActive);
    assert_eq!(snap.timer_status, LiveTimerStatus::Running);
    assert_eq!(snap.timer_anchor_server_ms, Some(1_005_000));
    assert_eq!(snap.pos_order_status.as_deref(), Some("BREWING"));

    // 4. Pause after 45.250s -> accumulated must be exact 45250 ms
    assert!(apply_mirror_command(&mut snap, &BrewerMirrorCommand::TimerPause, 1_050_250));
    assert_eq!(snap.phase, LiveBrewMirrorPhase::BrewingPaused);
    assert_eq!(snap.timer_status, LiveTimerStatus::Paused);
    assert_eq!(snap.timer_accumulated_ms, 45_250);
    assert_eq!(snap.timer_anchor_server_ms, None);

    // 5. Resume at 1_060_000 & Complete at 1_150_000 -> total 135250 ms, POS READY_TO_DELIVER
    assert!(apply_mirror_command(&mut snap, &BrewerMirrorCommand::TimerStartOrResume, 1_060_000));
    assert!(apply_mirror_command(
        &mut snap,
        &BrewerMirrorCommand::CompleteAndServe {
            final_weight_g: 225.4,
            accuracy_score_pct: Some(98.5),
        },
        1_150_000,
    ));
    assert_eq!(snap.phase, LiveBrewMirrorPhase::ServingReadyPassport);
    assert_eq!(snap.timer_status, LiveTimerStatus::Finished);
    assert_eq!(snap.timer_accumulated_ms, 135_250);
    assert_eq!(snap.pos_order_status.as_deref(), Some("READY_TO_DELIVER"));

    // Broadcast fan-out check
    let hub = MirrorStateHub::new(None);
    let mut rx = hub.subscribe_session(sid);
    hub.publish_snapshot(snap.clone());
    let received = rx.recv().await.expect("mirror frame");
    assert!(matches!(*received, MirrorDownstreamFrame::MirrorSnapshot(_)));
}
