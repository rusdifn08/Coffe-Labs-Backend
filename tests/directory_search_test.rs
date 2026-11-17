use std::{collections::HashSet, sync::Arc, time::Instant};

use axum::http::StatusCode;
use coffe_labs_backend::{
    core::{
        cache::AppCaches,
        error::AppError,
        migrations::{notifications_schema, profile_feed_schema, social_graph_schema},
        redis::{redis_get_bounded, redis_set_ex_bounded, RedisManager},
    },
    modules::social_feed::{
        error::SocialFeedError,
        repositories::{directory_repo::normalize_search_query, profile_projection_repo},
    },
};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[test]
fn directory_query_normalization_strips_at_prefix_and_trims() {
    assert_eq!(normalize_search_query(None), "");
    assert_eq!(normalize_search_query(Some("   ")), "");
    assert_eq!(normalize_search_query(Some("  @RusdiFn08 ")), "rusdifn08");
    assert_eq!(normalize_search_query(Some("@@@risaha15")), "risaha15");
    assert_eq!(normalize_search_query(Some("Ryan Wibawa")), "ryan wibawa");
}

#[tokio::test]
async fn app_caches_directory_and_viewer_followed_l1_work() {
    let caches = AppCaches::new();
    let payload = Arc::new(json!({ "items": [], "has_more": false }));
    caches
        .directory_l1
        .insert("rusdi:30".to_string(), Arc::clone(&payload))
        .await;
    assert!(caches.directory_l1.get("rusdi:30").await.is_some());

    let viewer_id = Uuid::new_v4();
    let followed = Arc::new(HashSet::from([Uuid::new_v4()]));
    caches
        .viewer_followed_l1
        .insert(viewer_id, Arc::clone(&followed))
        .await;
    assert_eq!(
        caches
            .viewer_followed_l1
            .get(&viewer_id)
            .await
            .map(|s| s.len()),
        Some(1)
    );
    caches.viewer_followed_l1.invalidate(&viewer_id).await;
    assert!(caches.viewer_followed_l1.get(&viewer_id).await.is_none());
}

#[tokio::test]
async fn bounded_redis_helpers_short_circuit_immediately_when_disconnected() {
    let mgr = RedisManager::disabled().expect("disabled redis manager");
    assert!(!mgr.is_connected());
    let started = Instant::now();
    let got = redis_get_bounded(&mgr.pool, "social:profile:test").await;
    redis_set_ex_bounded(&mgr.pool, "social:profile:test", "{}".into(), 60).await;
    assert!(got.is_none());
    assert!(
        started.elapsed().as_millis() < 100,
        "Redis L2 disconnected must return immediately without stalling request latency"
    );
}

#[tokio::test]
async fn resolve_target_user_id_supports_me_and_rejects_empty_without_db_hit() {
    let lazy_pg = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgresql://user:pass@localhost:5432/coffe_labs_test")
        .expect("lazy pg pool");

    let me_id = Uuid::new_v4();
    let resolved = profile_projection_repo::resolve_target_user_id(&lazy_pg, "me", Some(me_id))
        .await
        .expect("resolve me");
    assert_eq!(resolved, me_id);

    let err_empty = profile_projection_repo::resolve_target_user_id(&lazy_pg, "  @ ", Some(me_id))
        .await
        .expect_err("empty handle should fail fast with ProfileNotFound");
    assert!(matches!(err_empty, SocialFeedError::ProfileNotFound));

    let app_err: AppError = err_empty.into();
    let resp = axum::response::IntoResponse::into_response(app_err);
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn migrations_include_user_follows_status_legacy_username_backfill_trgm_and_inbox_user_id() {
    let graph_sql = social_graph_schema::statements().join("\n");
    assert!(graph_sql.contains("ALTER TABLE user_follows"));
    assert!(graph_sql.contains("ADD COLUMN IF NOT EXISTS status"));
    assert!(graph_sql.contains("follower_username"));
    assert!(graph_sql.contains("following_username"));

    let profile_sql = profile_feed_schema::statements().join("\n");
    assert!(profile_sql.contains("idx_profile_proj_lower_username"));
    assert!(profile_sql.contains("idx_profile_proj_trgm_search"));

    let notif_sql = notifications_schema::statements().join("\n");
    assert!(notif_sql.contains("user_id UUID NOT NULL"));
    assert!(notif_sql.contains("RENAME COLUMN recipient_id TO user_id"));
}
