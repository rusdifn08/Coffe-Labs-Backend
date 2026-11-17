use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use coffe_labs_backend::{
    app,
    core::{
        cache::AppCaches, config::AppConfig, fcm_oauth::FcmOAuthManager, jwt::JwtManager,
        middleware::SecurityRateLimiter, migrations::barista_toolkit_schema,
        rabbitmq::RabbitMqClient, redis::RedisManager, storage::CloudinaryStorage,
        telemetry::TelemetryRegistry, AppState,
    },
    modules::{
        barista_toolkit::{
            dto::BrewDoctorDiagnoseRequest,
            services::{
                brew_doctor_engine::diagnose_extraction,
                drm_crypto_service::{
                    decrypt_clbook_package, derive_device_wrapping_key, resolved_drm_secret,
                    unwrap_master_cek, wrap_cek_for_device,
                },
                game_brew_service::evaluate_game_brew_submission,
                lms_progress_service::{
                    decode_bitmap_13, evaluate_completion, grade_quiz_answers, merge_bitmap_or,
                    popcount_100bit, sign_hls_manifest_url, BITMAP_BYTES_LEN,
                },
            },
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
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use serde_json::{json, Value};
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
    }
}

#[test]
fn toolkit_migration_has_all_tables_and_valid_ddl() {
    let sql = barista_toolkit_schema::statements().join("\n");
    for name in [
        "course_difficulty_level",
        "lms_lesson_kind",
        "lms_courses",
        "lms_lessons",
        "user_lms_lesson_progress",
        "toolkit_ebooks",
        "user_ebook_progress",
        "game_brew_chapters",
        "user_game_brew_progress",
    ] {
        assert!(sql.contains(name), "Missing table/type: {name}");
    }
}

#[test]
fn aes_256_gcm_and_hkdf_unwrap_decrypts_python_clbook_asset() {
    let seed_raw = include_str!("../storage/ebooks_catalog_seed.json");
    let entries: Vec<Value> = serde_json::from_str(seed_raw).expect("seed json");
    let first = &entries[0];
    let book_id = Uuid::parse_str(first["id"].as_str().expect("id")).expect("uuid");
    let book_code = first["book_code"].as_str().expect("code");
    let wrapped_bytes = hex::decode(first["wrapped_master_cek_hex"].as_str().expect("hex")).expect("hex");

    let cek = unwrap_master_cek(&wrapped_bytes, book_code, &resolved_drm_secret()).expect("unwrap master cek");
    let client_ikm = b"device-keystore-random-32b-secret";
    let device_id = "android-test-device-01";
    let user_id = Uuid::new_v4();

    let (wrapped_b64, nonce_b64, aad_ctx) =
        wrap_cek_for_device(&cek, device_id, user_id, book_id, client_ikm).expect("wrap device");

    let (k_wrap, _) = derive_device_wrapping_key(client_ikm, device_id, user_id, book_id).expect("derive");
    let cipher = Aes256Gcm::new(&k_wrap.into());
    let ct = B64.decode(wrapped_b64).expect("ct b64");
    let nonce_bytes = B64.decode(nonce_b64).expect("nonce b64");
    let unwrapped_cek = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), Payload { msg: &ct, aad: aad_ctx.as_bytes() })
        .expect("client unwrap");
    let mut k_book = [0u8; 32];
    k_book.copy_from_slice(&unwrapped_cek);

    let mut clbook_blob = std::fs::read("storage/ebooks_clbook/CL-BOOK-01.clbook").expect("clbook file");
    let pdf_bytes = decrypt_clbook_package(&k_book, &clbook_blob).expect("decrypt clbook");
    assert!(pdf_bytes.starts_with(b"%PDF-"), "Decrypted payload must be valid PDF header");

    // Verify 1-byte tampering fails GCM authentication tag
    let last_idx = clbook_blob.len() - 1;
    clbook_blob[last_idx] ^= 0xFF;
    assert!(decrypt_clbook_package(&k_book, &clbook_blob).is_err());
}

#[test]
fn lms_100_bit_segment_bitmask_anti_skip_and_hls_signing_work() {
    let mut first_half = [0u8; BITMAP_BYTES_LEN];
    for i in 0..50 {
        first_half[i / 8] |= 1 << (i % 8);
    }
    let mut second_slice = [0u8; BITMAP_BYTES_LEN];
    for i in 45..88 {
        second_slice[i / 8] |= 1 << (i % 8);
    }
    // Set unused bit 102 in byte 12 to confirm masking prevents overcounting > 100
    second_slice[12] |= 0b1000_0000;

    let merged = merge_bitmap_or(&first_half, &second_slice);
    assert_eq!(popcount_100bit(&merged), 88);

    let roundtrip = decode_bitmap_13(&B64.encode(merged));
    assert_eq!(popcount_100bit(&roundtrip), 88);

    let quiz = json!([
        {"q": "Q1", "options": ["A", "B"], "answer_idx": 1},
        {"q": "Q2", "options": ["A", "B"], "answer_idx": 0}
    ]);
    assert_eq!(grade_quiz_answers(&quiz, &[1, 0]), Some(100));
    assert_eq!(grade_quiz_answers(&quiz, &[0, 0]), Some(50));
    assert!(evaluate_completion(88, &quiz, Some(100)));
    assert!(!evaluate_completion(84, &quiz, Some(100)));
    assert!(!evaluate_completion(95, &quiz, Some(50)));

    let (signed, _exp) = sign_hls_manifest_url("courses/v60/lesson_01.m3u8", Uuid::new_v4(), "hls-secret");
    assert!(signed.contains("sig="));
}

#[test]
fn brew_doctor_matrix_and_game_brew_evaluator_compute_accurately() {
    let sour_req = BrewDoctorDiagnoseRequest {
        symptom_tags: vec!["sour".to_string(), "quick_finish".to_string()],
        dose_g: 15.0,
        water_ml: 225.0,
        temp_c: 90.0,
        target_sec: 145,
        actual_sec: 115,
    };
    let sour_res = diagnose_extraction(&sour_req);
    assert_eq!(sour_res.diagnosis_code, "UNDER_EXTRACTED");
    assert_eq!(sour_res.grind_clicks_delta, -2);
    assert_eq!(sour_res.temp_c_delta, 2);

    let ch_req = BrewDoctorDiagnoseRequest {
        symptom_tags: vec!["sour".to_string(), "bitter".to_string()],
        dose_g: 15.0,
        water_ml: 225.0,
        temp_c: 92.0,
        target_sec: 145,
        actual_sec: 145,
    };
    assert_eq!(diagnose_extraction(&ch_req).diagnosis_code, "UNEVEN_CHANNELING");

    let eval = evaluate_game_brew_submission(1, 94.0, 20.1, Some(95.0), 0, 200);
    assert_eq!(eval.stars_earned, 3);
    assert_eq!(eval.unlocked_next_chapter, Some(2));
    assert_eq!(eval.xp_awarded, 200);
}

#[tokio::test]
async fn router_serves_brew_doctor_and_protects_authenticated_toolkit_endpoints() {
    let state = offline_state().await;
    let jwt = state.jwt.clone();
    let router = app::build_app(state);

    let unauth = router
        .clone()
        .oneshot(Request::builder().uri("/api/v1/toolkit/ebooks").body(Body::empty()).expect("req"))
        .await
        .expect("resp");
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    let uid = Uuid::new_v4();
    let tok = jwt.generate_user_token(uid, "barista", "user", "Trial", "approved").expect("jwt");
    let diag_res = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/toolkit/brew-doctor/diagnose")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "symptom_tags": ["bitter", "dry"],
                        "dose_g": 15.0,
                        "water_ml": 225.0,
                        "temp_c": 93.0,
                        "target_sec": 145,
                        "actual_sec": 200
                    })
                    .to_string(),
                ))
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(diag_res.status(), StatusCode::OK);
    let body = to_bytes(diag_res.into_body(), 64 * 1024).await.expect("bytes");
    let v: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(v["data"]["diagnosis_code"], "OVER_EXTRACTED");
    assert_eq!(v["data"]["grind_clicks_delta"], 2);
}

#[tokio::test]
async fn cloudinary_signer_bitmask_v3_and_object_storage_routes_work() {
    use coffe_labs_backend::core::config::parse_cloudinary_url;
    use coffe_labs_backend::modules::barista_toolkit::{
        bitmask_math::{merge_100bit_segments, BITMAP_BYTES_LEN as BM_LEN},
        cloudinary_signer::{
            generate_hls_stream_url, generate_optimized_image_url, generate_signed_raw_url,
        },
    };

    let sql = barista_toolkit_schema::statements().join("\n");
    assert!(sql.contains("ebook_chapters_ast"), "v3 AST table missing");
    assert!(sql.contains("coffee_dictionary_entries"), "v3 dictionary table missing");

    let creds = parse_cloudinary_url("cloudinary://11223344:super_api_secret@irn72dwa")
        .expect("valid cloudinary url");
    let raw_signed = generate_signed_raw_url(&creds, "coffeelabs/ebooks/CL-BOOK-01/CL-BOOK-01_id.epub", 3600);
    assert!(raw_signed.starts_with("https://res.cloudinary.com/irn72dwa/raw/authenticated/s--"));
    assert!(raw_signed.contains("/exp_"));

    let opt_img = generate_optimized_image_url(Some(&creds), "coffeelabs/ebooks/CL-BOOK-01/cover", 900);
    assert!(opt_img.contains("f_auto,q_auto:best,dpr_auto,w_900"));

    let hls = generate_hls_stream_url(Some(&creds), "coffeelabs/lms/bab1/sub_1_1_bcc.m3u8");
    assert_eq!(hls, "https://res.cloudinary.com/irn72dwa/video/upload/sp_auto/coffeelabs/lms/bab1/sub_1_1_bcc.m3u8");

    let mut inc = [0u8; BM_LEN];
    for i in 0..90 {
        inc[i / 8] |= 1 << (i % 8);
    }
    let (_, pct) = merge_100bit_segments(&[0u8; BM_LEN], &inc);
    assert_eq!(pct, 90);

    // Verify Object Storage endpoint serves compiled WebP & rejects directory traversal
    let state = offline_state().await;
    let router = app::build_app(state);

    let ok_res = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/toolkit/object-storage/coffeelabs/ebooks/CL-BOOK-01/cover.webp")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(ok_res.status(), StatusCode::OK);
    let webp_bytes = to_bytes(ok_res.into_body(), 2 * 1024 * 1024).await.expect("bytes");
    assert!(webp_bytes.starts_with(b"RIFF"));

    let traversal_res = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/toolkit/object-storage/coffeelabs/../.env")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("resp");
    assert_eq!(traversal_res.status(), StatusCode::BAD_REQUEST);

    // Verify generated LMS 9-Bab / 30-Sub-bab & Dictionary seeds
    let lms_v: Value = serde_json::from_str(include_str!("../storage/lms_manual_brew_seed.json")).expect("lms seed");
    let courses = lms_v["courses"].as_array().expect("courses");
    assert_eq!(courses.len(), 9);
    let total_lessons: usize = courses.iter().map(|c| c["lessons"].as_array().map(|a| a.len()).unwrap_or(0)).sum();
    assert_eq!(total_lessons, 30);

    let dict_v: Vec<Value> = serde_json::from_str(include_str!("../storage/coffee_dictionary_seed.json")).expect("dict seed");
    assert!(dict_v.len() >= 110, "expected 110+ dictionary entries, got {}", dict_v.len());
}
