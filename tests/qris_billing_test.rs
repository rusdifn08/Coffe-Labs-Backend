use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use secrecy::SecretString;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::RwLock;
use tower::ServiceExt;

use coffe_labs_backend::{
    app::build_app,
    core::{
        cache::AppCaches,
        config::AppConfig,
        fcm_oauth::FcmOAuthManager,
        jwt::JwtManager,
        middleware::SecurityRateLimiter,
        migrations::qris_billing_schema,
        rabbitmq::RabbitMqClient,
        redis::RedisManager,
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
        live_brew::services::live_telemetry_hub::LiveTelemetryHub,
        monitoring::MonitoringEngine,
        notifications::services::producer_service::NotificationProducerService,
        platform::models::AppVersionConfig,
        qris_billing::{
            emvco_qris::{
                build_dynamic_qris, compute_crc16_ccitt_false, extract_merchant_meta,
                parse_emvco_tlv, verify_qris_crc16, DEFAULT_COFFEELABS_STATIC_QRIS,
            },
            services::{
                bridge_security_guard::{
                    compute_bridge_signature_hex, compute_mutation_fingerprint,
                    parse_indonesian_rupiah_amount, verify_bridge_request,
                },
                email_imap_watcher_worker::is_verified_mandiri_email,
            },
        },
        social_feed::services::profile_cqrs_service::ProfileCqrsService,
        social_graph::services::{
            graph_command_service::GraphCommandService,
            graph_query_service::GraphQueryService,
            redis_graph_cache::RedisGraphCache,
        },
    },
};

async fn build_test_app_state() -> AppState {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://test:test@localhost:5432/test")
        .expect("lazy pool");
    let config = AppConfig {
        database_url: "postgres://test:test@localhost:5432/test".into(),
        redis_url: String::new(),
        rabbitmq_url: None,
        google_application_credentials: None,
        fcm_project_id: "coffe-labs".into(),
        cloudinary_url: None,
        geoapify_api_key: None,
        jwt_secret: SecretString::from("test-super-secret-jwt-key-minimum-32-bytes!!".to_string()),
        jwt_expiration_seconds: 86_400,
        port: 3000,
        environment: "test".into(),
        rate_limit_per_second: 500,
        rate_limit_burst: 5000,
    };
    let jwt = JwtManager::new(config.jwt_secret_str(), config.jwt_expiration_seconds);
    let caches = AppCaches::new();
    let redis = RedisManager::disabled().expect("redis");
    let chat_hub = Arc::new(ConnectionHub::new());
    let chat_bridge = RedisPubSubBridge::from_shared(Arc::clone(&chat_hub), None, None).await;
    let social_profile_cqrs = Arc::new(ProfileCqrsService::with_l1_cache(
        None,
        caches.profile_l1.clone(),
    ));
    let (club_cache, secret_join_service, taste_profiler) =
        coffe_labs_backend::core::init_module35_engines(&pool, None, &config);
    let graph_cache = Arc::new(RedisGraphCache::new(None));
    let follow_notif = Arc::new(
        coffe_labs_backend::modules::social_graph::services::follow_notif_dispatcher::FollowNotifDispatcher::new(
            chat_bridge.clone(),
            social_profile_cqrs.clone(),
        ),
    );
    let social_graph_cmd = Arc::new(GraphCommandService::new(graph_cache.clone(), follow_notif));
    let social_graph_query = Arc::new(GraphQueryService::new(graph_cache));
    let monitoring = MonitoringEngine::init(None);
    let live_brew_hub = Arc::new(LiveTelemetryHub::new(None));

    AppState {
        pool,
        config,
        jwt,
        telemetry: TelemetryRegistry::new(),
        rate_limiter: SecurityRateLimiter::new(500, 5000),
        version_cache: Arc::new(RwLock::new(AppVersionConfig::fallback_default())),
        chat_hub,
        chat_bridge,
        chat_presence: Arc::new(PresenceService::new()),
        chat_messages: Arc::new(MessageService::with_member_cache(caches.chat_member_l1.clone())),
        chat_moderation: Arc::new(ModerationGuard::new()),
        social_profile_cqrs,
        social_graph_cmd,
        social_graph_query,
        notifications: NotificationProducerService::noop_for_tests(),
        rabbitmq: RabbitMqClient::disabled(),
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
fn emvco_qris_converts_real_livin_merchant_coffeelabs_qris_with_exact_crc16() {
    assert_eq!(verify_qris_crc16(DEFAULT_COFFEELABS_STATIC_QRIS).ok(), Some(true));

    let meta = extract_merchant_meta(DEFAULT_COFFEELABS_STATIC_QRIS);
    assert_eq!(meta.merchant_name, "Coffeelabs");
    assert_eq!(meta.merchant_city, "Bandung (Kab)");
    assert_eq!(meta.acquirer_code, "93600008");
    assert_eq!(meta.nmid, "ID1023277752888");

    // 1. Pro Lifetime Promo Akhir 2026: Rp 99.317
    let pro_qris = build_dynamic_qris(DEFAULT_COFFEELABS_STATIC_QRIS, 99_317)
        .expect("dynamic Pro Lifetime QRIS must build");
    assert!(pro_qris.starts_with("000201010212"));
    assert!(pro_qris.contains("5405993175802ID"));
    assert!(pro_qris.contains("5910Coffeelabs"));
    assert_eq!(verify_qris_crc16(&pro_qris).ok(), Some(true));

    let body_up_to_6304 = &pro_qris[..pro_qris.len() - 4];
    let expected_crc = format!("{:04X}", compute_crc16_ccitt_false(body_up_to_6304.as_bytes()));
    assert_eq!(&pro_qris[pro_qris.len() - 4..], expected_crc);

    // 2. Business Lifetime Promo Akhir 2026: Rp 249.842
    let biz_qris = build_dynamic_qris(DEFAULT_COFFEELABS_STATIC_QRIS, 249_842)
        .expect("dynamic Business Lifetime QRIS must build");
    let tags = parse_emvco_tlv(&biz_qris).expect("TLV must parse cleanly");
    assert!(tags.iter().any(|(k, v)| k == "01" && v == "12"));
    assert!(tags.iter().any(|(k, v)| k == "54" && v == "249842"));
    assert!(tags.iter().any(|(k, v)| k == "59" && v == "Coffeelabs"));
    assert_eq!(verify_qris_crc16(&biz_qris).ok(), Some(true));
}

#[tokio::test]
async fn bridge_security_guard_verifies_hmac_rejects_replay_and_parses_rupiah() {
    let secret = "super-secret-livin-bridge-key-2026";
    let body = br#"{"package_name":"id.bmri.livinmerchant","raw_notification_text":"Pembayaran QRIS sebesar Rp49.317 berhasil diterima"}"#;
    let now_ms = Utc::now().timestamp_millis();
    let nonce = format!("nonce-{}", uuid::Uuid::new_v4());
    let sig = compute_bridge_signature_hex(secret, now_ms, &nonce, body).expect("sig");

    // Valid first request passes
    let ok_res = verify_bridge_request(
        None,
        secret,
        Some(&now_ms.to_string()),
        Some(&nonce),
        Some(&sig),
        body,
    )
    .await;
    assert!(ok_res.is_ok());

    // Exact replay with same nonce MUST be rejected
    let replay_res = verify_bridge_request(
        None,
        secret,
        Some(&now_ms.to_string()),
        Some(&nonce),
        Some(&sig),
        body,
    )
    .await;
    assert!(replay_res.is_err());

    // Tampered body MUST be rejected
    let bad_sig_res = verify_bridge_request(
        None,
        secret,
        Some(&now_ms.to_string()),
        Some(&format!("nonce-{}", uuid::Uuid::new_v4())),
        Some(&sig),
        br#"{"amount_idr":1}"#,
    )
    .await;
    assert!(bad_sig_res.is_err());

    // Indonesian Rupiah notification extraction
    assert_eq!(
        parse_indonesian_rupiah_amount("Pembayaran QRIS sebesar Rp49.317 berhasil diterima di Coffeelabs"),
        Some(49_317)
    );
    assert_eq!(
        parse_indonesian_rupiah_amount("Dana masuk Rp 149.842,00 dari QRIS"),
        Some(149_842)
    );
    assert_eq!(
        parse_indonesian_rupiah_amount("Transaksi IDR 1.490.512 sukses"),
        Some(1_490_512)
    );

    // Idempotent fingerprint + DKIM/SPF verification
    let fp1 = compute_mutation_fingerprint(49_317, Some("RRN998877"), 1_700_000_000);
    let fp2 = compute_mutation_fingerprint(49_317, Some("RRN998877"), 1_700_000_099);
    assert_eq!(fp1, fp2);

    assert!(is_verified_mandiri_email(
        "From: noreply@bankmandiri.co.id\nAuthentication-Results: mx.google.com; dkim=pass; spf=pass\nPembayaran Rp49.317"
    ));
    assert!(!is_verified_mandiri_email(
        "From: spoof@evil.com\nAuthentication-Results: mx.google.com; dkim=fail; spf=fail\nPembayaran Rp49.317"
    ));
}

#[test]
fn qris_billing_migration_contains_all_tables_and_partial_unique_index() {
    let joined = qris_billing_schema::statements().join("\n");
    assert!(joined.contains("CREATE TYPE qris_invoice_status"));
    assert!(joined.contains("CREATE TYPE subscription_plan_code"));
    assert!(joined.contains("subscription_plan_catalog"));
    assert!(joined.contains("qris_subscription_invoices"));
    assert!(joined.contains("uq_qris_active_pending_amount"));
    assert!(joined.contains("qris_payment_mutations"));
}

#[tokio::test]
async fn router_protects_checkout_admin_and_bridge_endpoints() {
    let state = build_test_app_state().await;
    let app = build_app(state.clone());

    // Checkout requires JWT
    let res_checkout = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/billing/qris/checkout")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"plan_code":"PRO_MONTHLY"}"#))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res_checkout.status(), StatusCode::UNAUTHORIZED);

    // Bridge ingest without HMAC headers must return 401 UNAUTHORIZED
    let res_bridge = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/billing/qris/bridge-ingest")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"raw_notification_text":"Rp49.317"}"#))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res_bridge.status(), StatusCode::UNAUTHORIZED);

    // Admin endpoint rejects non-admin JWT with 403 FORBIDDEN
    let user_token = state
        .jwt
        .generate_user_token(uuid::Uuid::new_v4(), "barista1", "user", "trial", "approved")
        .expect("jwt");
    let res_admin = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/billing/qris/admin/unmatched-mutations")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(res_admin.status(), StatusCode::FORBIDDEN);
}
