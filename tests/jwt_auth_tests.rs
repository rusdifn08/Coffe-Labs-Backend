use axum::{
    body::{to_bytes, Body},
    extract::FromRef,
    http::{header, Request, StatusCode},
    routing::get,
    Json, Router,
};
use coffe_labs_backend::core::{
    auth_extractor::{AuthenticatedUser, OptionalAuthenticatedUser},
    config::{parse_jwt_expiration, parse_jwt_secret},
    error::AppError,
    jwt::JwtManager,
};
use secrecy::ExposeSecret;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone)]
struct JwtTestState {
    jwt: JwtManager,
}

impl FromRef<JwtTestState> for JwtManager {
    fn from_ref(state: &JwtTestState) -> Self {
        state.jwt.clone()
    }
}

async fn protected_handler(auth: AuthenticatedUser) -> Json<Value> {
    Json(json!({
        "user_id": auth.user_id,
        "account_tier": auth.account_tier,
        "username": auth.claims.username,
    }))
}

async fn optional_handler(OptionalAuthenticatedUser(opt): OptionalAuthenticatedUser) -> Json<Value> {
    match opt {
        Some(auth) => Json(json!({ "authenticated": true, "user_id": auth.user_id })),
        None => Json(json!({ "authenticated": false })),
    }
}

fn test_router(jwt: JwtManager) -> Router {
    Router::new()
        .route("/api/v1/protected", get(protected_handler))
        .route("/api/v1/chat/ws", get(protected_handler))
        .route("/api/v1/optional", get(optional_handler))
        .with_state(JwtTestState { jwt })
}

#[test]
fn config_jwt_parsers_enforce_required_secret_and_safe_defaults() {
    assert!(parse_jwt_secret(None).is_err());
    assert!(parse_jwt_secret(Some("   ".to_string())).is_err());

    let valid = parse_jwt_secret(Some(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
    ))
    .expect("valid 64-char hex JWT_SECRET");
    assert_eq!(valid.expose_secret().len(), 64);

    assert_eq!(parse_jwt_expiration(None), 86_400);
    assert_eq!(parse_jwt_expiration(Some("3600".to_string())), 3600);
    assert_eq!(parse_jwt_expiration(Some("0".to_string())), 86_400);
    assert_eq!(parse_jwt_expiration(Some("invalid".to_string())), 86_400);
}

#[test]
fn jwt_manager_roundtrip_wrong_secret_and_expiry_checks() {
    let secret_a = "coffee_labs_test_secret_alpha_minimum_32_bytes!";
    let secret_b = "coffee_labs_test_secret_bravo_minimum_32_bytes!";
    let manager_a = JwtManager::new(secret_a, 3600);
    let manager_b = JwtManager::new(secret_b, 3600);

    let debug_output = format!("{manager_a:?}");
    assert!(!debug_output.contains(secret_a));

    let uid = Uuid::new_v4();
    let token = manager_a
        .generate_token(uid, "Pro")
        .expect("token generation must succeed");

    let claims = manager_a
        .verify_token(&token)
        .expect("matching secret verifies token");
    assert_eq!(claims.sub, uid);
    assert_eq!(claims.tier, "Pro");
    assert!(claims.exp > claims.iat);

    match manager_b.verify_token(&token) {
        Err(AppError::Unauthorized(_)) => {}
        other => panic!("expected Unauthorized for wrong secret, got: {other:?}"),
    }

    let expired_token = manager_a
        .generate_token_with_ttl(uid, "Trial", "user", "Trial", "active", -120)
        .expect("expired token fixture signs");
    match manager_a.verify_token(&expired_token) {
        Err(AppError::Unauthorized(_)) => {}
        other => panic!("expected Unauthorized for expired token, got: {other:?}"),
    }
}

#[tokio::test]
async fn authenticated_user_extracts_from_bearer_header_and_ws_query_param() {
    let jwt = JwtManager::new("coffee_labs_dual_extractor_secret_key_2026_v3", 3600);
    let app = test_router(jwt.clone());
    let uid = Uuid::new_v4();
    let token = jwt
        .generate_user_token(uid, "barista_jkt", "user", "Business", "active")
        .expect("generate user token");

    // 1. Authorization: Bearer <token> header
    let bearer_req = Request::builder()
        .uri("/api/v1/protected")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .expect("build bearer req");
    let bearer_resp = app.clone().oneshot(bearer_req).await.expect("oneshot");
    assert_eq!(bearer_resp.status(), StatusCode::OK);
    let bearer_bytes = to_bytes(bearer_resp.into_body(), 64 * 1024).await.expect("body");
    let bearer_json: Value = serde_json::from_slice(&bearer_bytes).expect("json");
    assert_eq!(bearer_json["user_id"], uid.to_string());
    assert_eq!(bearer_json["account_tier"], "Business");
    assert_eq!(bearer_json["username"], "barista_jkt");

    // 2. WebSocket fallback ?token=<jwt> query parameter
    let ws_query_req = Request::builder()
        .uri(format!("/api/v1/chat/ws?token={token}&format=json"))
        .body(Body::empty())
        .expect("build ws query req");
    let ws_resp = app.clone().oneshot(ws_query_req).await.expect("oneshot");
    assert_eq!(ws_resp.status(), StatusCode::OK);
    let ws_bytes = to_bytes(ws_resp.into_body(), 64 * 1024).await.expect("body");
    let ws_json: Value = serde_json::from_slice(&ws_bytes).expect("json");
    assert_eq!(ws_json["user_id"], uid.to_string());
    assert_eq!(ws_json["account_tier"], "Business");

    // 3. Invalid or missing token -> 401 Unauthorized
    let missing_req = Request::builder()
        .uri("/api/v1/chat/ws")
        .body(Body::empty())
        .expect("build missing req");
    assert_eq!(
        app.clone().oneshot(missing_req).await.expect("oneshot").status(),
        StatusCode::UNAUTHORIZED
    );

    let tampered_req = Request::builder()
        .uri("/api/v1/chat/ws?token=tampered.invalid.jwt")
        .body(Body::empty())
        .expect("build tampered req");
    assert_eq!(
        app.clone().oneshot(tampered_req).await.expect("oneshot").status(),
        StatusCode::UNAUTHORIZED
    );

    // 4. Optional extractor cleanly falls back to unauthenticated on invalid or missing token
    let opt_unauth = Request::builder()
        .uri("/api/v1/optional")
        .body(Body::empty())
        .expect("build opt req");
    let opt_resp = app.oneshot(opt_unauth).await.expect("oneshot");
    assert_eq!(opt_resp.status(), StatusCode::OK);
    let opt_json: Value =
        serde_json::from_slice(&to_bytes(opt_resp.into_body(), 64 * 1024).await.expect("body"))
            .expect("json");
    assert_eq!(opt_json["authenticated"], false);
}
