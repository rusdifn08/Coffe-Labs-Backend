use std::sync::Arc;

use coffe_labs_backend::core::{
    cache::{
        AppCaches, CHAT_MEMBER_CACHE_MAX_CAPACITY, CHAT_MEMBER_CACHE_TTL_SECS,
        FCM_TOKEN_CACHE_MAX_CAPACITY, FCM_TOKEN_CACHE_TTL_SECS, PROFILE_CACHE_MAX_CAPACITY,
        PROFILE_CACHE_TTL_SECS as PROFILE_L1_TTL_SECS,
    },
    fcm_oauth::FcmOAuthManager,
    redis::{
        build_reconnect_policy, sanitize_redis_url, RedisManager, PROFILE_CACHE_TTL_SECS,
        REDIS_POOL_SIZE, REDIS_STREAM_MAXLEN,
    },
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn redis_constants_and_backoff_policy_match_free_tier_limits() {
    assert_eq!(REDIS_POOL_SIZE, 4);
    assert_eq!(REDIS_STREAM_MAXLEN, 1_000);
    assert_eq!(PROFILE_CACHE_TTL_SECS, 300);
    let _policy = build_reconnect_policy();
}

#[test]
fn sanitize_redis_url_masks_password_without_panic() {
    assert_eq!(
        sanitize_redis_url("redis://default:SuperSecret123@redis-cloud.example:18473"),
        "redis://***:***@redis-cloud.example:18473"
    );
    assert_eq!(
        sanitize_redis_url("rediss://user:pass@secure-redis.example:6380/0"),
        "rediss://***:***@secure-redis.example:6380/0"
    );
    assert_eq!(sanitize_redis_url("localhost:6379"), "[REDACTED_REDIS_URL]");
    assert_eq!(
        sanitize_redis_url("redis://127.0.0.1:6379"),
        "redis://127.0.0.1:6379"
    );
}

#[tokio::test]
async fn app_caches_enforces_strict_render_ram_limits_and_operations() {
    assert_eq!(PROFILE_CACHE_MAX_CAPACITY, 1_000);
    assert_eq!(PROFILE_L1_TTL_SECS, 3);
    assert_eq!(CHAT_MEMBER_CACHE_MAX_CAPACITY, 5_000);
    assert_eq!(CHAT_MEMBER_CACHE_TTL_SECS, 300);
    assert_eq!(FCM_TOKEN_CACHE_MAX_CAPACITY, 2);
    assert_eq!(FCM_TOKEN_CACHE_TTL_SECS, 55 * 60);

    let caches = AppCaches::new();
    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    caches
        .profile_l1
        .insert(user_id, Arc::new(json!({ "username": "barista_id" })))
        .await;
    caches.chat_member_l1.insert((room_id, user_id), true).await;
    caches
        .fcm_token_l1
        .insert("fcm_oauth_bearer".to_string(), "dry_run_oauth_token".to_string())
        .await;

    caches.profile_l1.run_pending_tasks().await;
    caches.chat_member_l1.run_pending_tasks().await;
    caches.fcm_token_l1.run_pending_tasks().await;

    assert!(caches.profile_l1.get(&user_id).await.is_some());
    assert_eq!(caches.chat_member_l1.get(&(room_id, user_id)).await, Some(true));
    assert_eq!(
        caches.fcm_token_l1.get("fcm_oauth_bearer").await.as_deref(),
        Some("dry_run_oauth_token")
    );
    assert_eq!(caches.entry_counts(), (1, 1, 1));
}

#[tokio::test]
async fn redis_manager_handles_missing_and_invalid_url_without_panic() {
    let disabled = RedisManager::disabled().expect("fallback RedisManager builds cleanly");
    assert!(!disabled.is_configured());
    assert!(!disabled.is_connected());
    assert!(disabled.active_pool().is_none());
    assert!(disabled.active_subscriber().is_none());
    assert!(disabled.active_client().is_none());

    let unset = RedisManager::connect_or_lazy("")
        .await
        .expect("empty url falls back cleanly");
    assert!(!unset.is_configured());
    assert!(!unset.is_connected());
    assert!(unset.active_pool().is_none());

    let invalid = RedisManager::connect_or_lazy("not-a-valid-redis-url")
        .await
        .expect("invalid url falls back without panic");
    assert!(!invalid.is_configured());
    assert!(!invalid.is_connected());
    assert!(invalid.active_pool().is_none());
}

#[tokio::test]
async fn fcm_oauth_manager_returns_domain_error_without_panic_when_unconfigured() {
    let caches = AppCaches::new();
    let mgr = FcmOAuthManager::from_config(Some("/nonexistent/firebase-service-account.json"), "coffe-labs");
    assert_eq!(mgr.resolved_project_id(), "coffe-labs");
    let err = mgr.get_valid_bearer_token(&caches).await;
    assert!(err.is_err());

    caches
        .fcm_token_l1
        .insert("fcm_oauth_bearer".to_string(), "cached_bearer".to_string())
        .await;
    caches.fcm_token_l1.run_pending_tasks().await;
    let cached = mgr.get_valid_bearer_token(&caches).await.expect("returns cached token");
    assert_eq!(cached, "cached_bearer");
}
