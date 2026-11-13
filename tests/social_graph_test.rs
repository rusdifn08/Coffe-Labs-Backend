use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use coffe_labs_backend::modules::{
    chat::{
        protocol::ServerFrame,
        services::{connection_hub::ConnectionHub, redis_pubsub_bridge::RedisPubSubBridge},
    },
    social_feed::services::profile_cqrs_service::ProfileCqrsService,
    social_graph::{
        error::SocialGraphError,
        models::{GraphCursor, RelationshipState},
        services::{
            follow_notif_dispatcher::FollowNotifDispatcher,
            graph_command_service::GraphCommandService,
            redis_graph_cache::RedisGraphCache,
        },
    },
};

#[test]
fn relationship_state_resolves_full_6_way_matrix() {
    let viewer = Uuid::new_v4();
    let target = Uuid::new_v4();

    assert_eq!(
        RelationshipState::resolve(Some(viewer), viewer, false, false, false),
        RelationshipState::SelfProfile
    );
    assert_eq!(
        RelationshipState::resolve(None, target, true, true, false),
        RelationshipState::None
    );
    assert_eq!(
        RelationshipState::resolve(Some(viewer), target, false, false, false),
        RelationshipState::None
    );
    assert_eq!(
        RelationshipState::resolve(Some(viewer), target, false, true, false),
        RelationshipState::FollowedByTarget
    );
    assert_eq!(
        RelationshipState::resolve(Some(viewer), target, false, false, true),
        RelationshipState::Requested
    );
    assert_eq!(
        RelationshipState::resolve(Some(viewer), target, false, true, true),
        RelationshipState::Requested
    );
    assert_eq!(
        RelationshipState::resolve(Some(viewer), target, true, false, false),
        RelationshipState::Following
    );
    assert_eq!(
        RelationshipState::resolve(Some(viewer), target, true, true, false),
        RelationshipState::Mutual
    );
}

#[test]
fn graph_cursor_encodes_and_decodes_microsecond_keyset_and_rejects_bad_input() {
    let now = Utc::now();
    let uid = Uuid::new_v4();
    let encoded = GraphCursor {
        created_at: now,
        user_id: uid,
    }
    .encode();
    let decoded = GraphCursor::decode(&encoded).expect("valid cursor should decode");
    assert_eq!(decoded.user_id, uid);
    assert_eq!(decoded.created_at.timestamp_micros(), now.timestamp_micros());

    assert!(GraphCursor::decode("!!!invalid_base64").is_err());
    assert!(GraphCursor::decode("bWlzc2luZ19waXBl").is_err());
}

#[test]
fn server_frame_social_graph_event_roundtrips_json() {
    let actor_id = Uuid::new_v4();
    let frame = ServerFrame::SocialGraphEvent {
        actor_id,
        actor_username: "ryan_wbrc".to_string(),
        actor_display_name: "Ryan Wibawa".to_string(),
        actor_avatar_url: Some("https://cdn.coffelabs.id/avatars/ryan.png".to_string()),
        state: "followed_by_target".to_string(),
        action: "follow".to_string(),
        message: "Ryan Wibawa mulai mengikuti Anda.".to_string(),
        created_at: Utc::now(),
    };

    let json = serde_json::to_string(&frame).expect("serialize social graph event");
    assert!(json.contains("\"type\":\"social_graph_event\""));
    assert!(json.contains("\"action\":\"follow\""));

    let back: ServerFrame = serde_json::from_str(&json).expect("deserialize social graph event");
    match back {
        ServerFrame::SocialGraphEvent {
            actor_id: d_id,
            actor_username,
            state,
            action,
            ..
        } => {
            assert_eq!(d_id, actor_id);
            assert_eq!(actor_username, "ryan_wbrc");
            assert_eq!(state, "followed_by_target");
            assert_eq!(action, "follow");
        }
        other => panic!("Unexpected frame variant: {other:?}"),
    }
}

#[tokio::test]
async fn rate_limiter_allows_15_actions_per_minute_per_user_and_dedups_notifs() {
    let hub = Arc::new(ConnectionHub::new());
    let bridge = RedisPubSubBridge::connect_or_local(hub, None).await;
    let cqrs = Arc::new(ProfileCqrsService::new(None));
    let notif = Arc::new(FollowNotifDispatcher::new(bridge, cqrs));
    let cache = Arc::new(RedisGraphCache::new(None));
    assert!(cache.redis_pool().is_none());

    let cmd = GraphCommandService::new(cache, notif.clone());
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    for i in 0..15 {
        assert!(
            cmd.enforce_rate_limit(user_a).is_ok(),
            "action #{i} should pass within 15/min quota"
        );
    }
    match cmd.enforce_rate_limit(user_a) {
        Err(SocialGraphError::TooManyRequests {
            retry_after_secs, ..
        }) => assert!(retry_after_secs >= 1),
        other => panic!("Expected TooManyRequests on 16th action, got {other:?}"),
    }
    // Separate user bucket remains unaffected
    assert!(cmd.enforce_rate_limit(user_b).is_ok());

    // 1-hour notification cooldown deduplication
    assert!(notif.try_acquire_cooldown(user_a, user_b).await);
    assert!(!notif.try_acquire_cooldown(user_a, user_b).await);
    // Reverse direction (accepting/folback) uses its own cooldown key
    assert!(notif.try_acquire_cooldown(user_b, user_a).await);
}
