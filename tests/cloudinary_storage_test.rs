use axum::{
    body::{to_bytes, Body},
    extract::FromRef,
    http::{header, Request, StatusCode},
};
use coffe_labs_backend::{
    core::{
        config::parse_cloudinary_url,
        security::JwtKeyRing,
        storage::{auto_optimize_url, CloudinaryStorage},
    },
    modules::media::routes::media_router,
};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(Clone)]
struct MediaTestState {
    jwt_keys: JwtKeyRing,
    cloudinary: CloudinaryStorage,
}

impl FromRef<MediaTestState> for JwtKeyRing {
    fn from_ref(state: &MediaTestState) -> Self {
        state.jwt_keys.clone()
    }
}

impl FromRef<MediaTestState> for CloudinaryStorage {
    fn from_ref(state: &MediaTestState) -> Self {
        state.cloudinary.clone()
    }
}

#[test]
fn parse_cloudinary_url_extracts_credentials_and_rejects_malformed_inputs() {
    let parsed = parse_cloudinary_url("  cloudinary://123456789:secret_with:colon@irn72dwa/  ")
        .expect("valid cloudinary URL");
    assert_eq!(parsed.api_key, "123456789");
    assert_eq!(parsed.api_secret.expose_secret(), "secret_with:colon");
    assert_eq!(parsed.cloud_name, "irn72dwa");

    let with_at_secret = parse_cloudinary_url("cloudinary://key1:sec@ret@irn72dwa?secure=true")
        .expect("handles @ inside api_secret");
    assert_eq!(with_at_secret.api_key, "key1");
    assert_eq!(with_at_secret.api_secret.expose_secret(), "sec@ret");
    assert_eq!(with_at_secret.cloud_name, "irn72dwa");

    for bad in [
        "",
        "https://key:secret@irn72dwa",
        "cloudinary://key:secret",
        "cloudinary://key_only@irn72dwa",
        "cloudinary://:secret@irn72dwa",
        "cloudinary://key:@irn72dwa",
        "cloudinary://key:secret@",
        "cloudinary://key:secret@bad.host/path",
        "cloudinary://key with space:secret@irn72dwa",
    ] {
        assert!(parse_cloudinary_url(bad).is_err(), "expected failure for: {bad}");
    }
}

#[test]
fn cloudinary_storage_generates_deterministic_sha1_signature_and_validates_inputs() {
    let storage = CloudinaryStorage::from_url("cloudinary://99887766:my_super_secret@irn72dwa")
        .expect("storage constructs cleanly");
    assert!(storage.is_configured());
    assert_eq!(storage.cloud_name(), Some("irn72dwa"));

    let debug_str = format!("{storage:?}");
    assert!(!debug_str.contains("my_super_secret"));
    assert!(!debug_str.contains("99887766"));

    let signed_with_pid = storage
        .generate_upload_signature_at(1_700_000_000, "social_posts", Some("brew_01"))
        .expect("signature with public_id");
    assert_eq!(signed_with_pid.timestamp, 1_700_000_000);
    assert_eq!(
        signed_with_pid.signature,
        "92d088852c75d58806ca463c2847edafd18d43cd"
    );
    assert_eq!(signed_with_pid.api_key, "99887766");
    assert_eq!(signed_with_pid.cloud_name, "irn72dwa");
    assert_eq!(
        signed_with_pid.upload_url,
        "https://api.cloudinary.com/v1_1/irn72dwa/image/upload"
    );
    assert_eq!(signed_with_pid.folder, "social_posts");
    assert_eq!(signed_with_pid.public_id.as_deref(), Some("brew_01"));

    let signed_folder_only = storage
        .generate_upload_signature_at(1_700_000_000, "avatars", Some("   "))
        .expect("empty whitespace public_id ignored cleanly");
    assert_eq!(
        signed_folder_only.signature,
        "30305c476cb32d309d17dfff9ee081edfe11e308"
    );
    assert!(signed_folder_only.public_id.is_none());

    let live = storage
        .generate_upload_signature("coffe_labs/brews", None)
        .expect("live signature succeeds");
    assert!(live.timestamp > 1_700_000_000);
    assert_eq!(live.signature.len(), 40);
    assert!(live.signature.chars().all(|c| c.is_ascii_hexdigit()));

    assert!(storage.generate_upload_signature("", None).is_err());
    assert!(storage
        .generate_upload_signature("avatars&overwrite=true", None)
        .is_err());
    assert!(storage
        .generate_upload_signature("avatars/../admin", None)
        .is_err());
    assert!(storage
        .generate_upload_signature("avatars", Some("pid=injected"))
        .is_err());

    let unconfigured = CloudinaryStorage::unconfigured();
    assert!(!unconfigured.is_configured());
    assert!(unconfigured
        .generate_upload_signature("avatars", None)
        .is_err());
}

#[test]
fn auto_optimize_url_inserts_f_auto_q_auto_idempotently() {
    assert_eq!(
        CloudinaryStorage::auto_optimize_url(
            "https://res.cloudinary.com/irn72dwa/image/upload/v1731500000/social/v60.jpg"
        ),
        "https://res.cloudinary.com/irn72dwa/image/upload/f_auto,q_auto/v1731500000/social/v60.jpg"
    );
    assert_eq!(
        auto_optimize_url(
            "https://res.cloudinary.com/irn72dwa/image/upload/c_fill,w_400/social/v60.jpg"
        ),
        "https://res.cloudinary.com/irn72dwa/image/upload/f_auto,q_auto/c_fill,w_400/social/v60.jpg"
    );
    assert_eq!(
        auto_optimize_url(
            "https://res.cloudinary.com/irn72dwa/image/upload/f_auto,q_auto/v1731500000/social/v60.jpg"
        ),
        "https://res.cloudinary.com/irn72dwa/image/upload/f_auto,q_auto/v1731500000/social/v60.jpg"
    );
    assert_eq!(
        auto_optimize_url(
            "https://res.cloudinary.com/irn72dwa/image/upload/avatars/f_auto,q_auto_photo.jpg"
        ),
        "https://res.cloudinary.com/irn72dwa/image/upload/f_auto,q_auto/avatars/f_auto,q_auto_photo.jpg"
    );
    assert_eq!(
        auto_optimize_url("https://cdn.example.com/images/latte.png"),
        "https://cdn.example.com/images/latte.png"
    );
    assert_eq!(auto_optimize_url("   "), "");
}

#[tokio::test]
async fn media_upload_signature_endpoint_requires_auth_and_returns_signed_payload() {
    let jwt_keys =
        JwtKeyRing::from_seed(&SecretString::from("test_seed_cloudinary_2026".to_string()))
            .expect("jwt keys");
    let cloudinary = CloudinaryStorage::from_url("cloudinary://api_123:sec_456@irn72dwa")
        .expect("cloudinary storage");
    let state = MediaTestState {
        jwt_keys: jwt_keys.clone(),
        cloudinary,
    };
    let app = media_router::<MediaTestState>().with_state(state);

    // 1. Unauthenticated request must return 401 Unauthorized
    let unauth_req = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload-signature")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "folder": "avatars" }).to_string()))
        .expect("build request");
    let unauth_resp = app.clone().oneshot(unauth_req).await.expect("oneshot");
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // 2. Authenticated valid request returns signed upload contract
    let token = jwt_keys
        .sign_access_token("user-1", "barista", "user", "Pro", "active")
        .expect("sign token");
    let auth_req = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload-signature")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "folder": "social_posts", "public_id": "thread_99" }).to_string(),
        ))
        .expect("build signed request");
    let auth_resp = app.clone().oneshot(auth_req).await.expect("oneshot");
    assert_eq!(auth_resp.status(), StatusCode::OK);

    let body_bytes = to_bytes(auth_resp.into_body(), 64 * 1024)
        .await
        .expect("read body");
    let json_body: Value = serde_json::from_slice(&body_bytes).expect("parse json");
    assert_eq!(json_body["success"], true);
    assert_eq!(json_body["data"]["cloud_name"], "irn72dwa");
    assert_eq!(json_body["data"]["api_key"], "api_123");
    assert_eq!(
        json_body["data"]["upload_url"],
        "https://api.cloudinary.com/v1_1/irn72dwa/image/upload"
    );
    assert_eq!(json_body["data"]["folder"], "social_posts");
    assert_eq!(json_body["data"]["public_id"], "thread_99");
    let sig = json_body["data"]["signature"].as_str().expect("signature str");
    assert_eq!(sig.len(), 40);

    // 3. Injection characters rejected cleanly with 400 Bad Request
    let bad_req = Request::builder()
        .method("POST")
        .uri("/api/v1/media/upload-signature")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({ "folder": "avatars&overwrite=true" }).to_string(),
        ))
        .expect("build bad request");
    let bad_resp = app.oneshot(bad_req).await.expect("oneshot");
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);
}
