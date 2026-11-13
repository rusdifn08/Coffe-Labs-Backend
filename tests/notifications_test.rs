use chrono::{TimeZone, Utc};
use coffe_labs_backend::modules::{
    chat::protocol::{ClientFrame, ServerFrame},
    notifications::{
        builders::{
            build_chat_message_event, build_mutual_follback_event, build_new_follower_event,
            build_streak_milestone_event, build_thread_like_coalesce_seed,
            build_thread_reply_event, build_tier_upgrade_event,
        },
        dto::NotificationCursor,
        events::{
            format_coalesced_actor_names, ActorSummary, NotificationActionButton,
            NotificationCategory,
        },
        models::NotificationPreferenceRow,
        services::{
            backoff::{
                compute_full_jitter_delay_ms, is_event_enabled_for_user, is_quiet_hours_now, R_MAX,
                T_BASE_MS, T_MAX_MS,
            },
            fcm_payload::build_fcm_v1_message,
            fcm_v1_gateway::{FcmDeliveryOutcome, FcmV1Gateway},
            producer_service::{NotificationProducerService, NOTIF_DLQ_KEY, NOTIF_STREAM_KEY},
        },
    },
};
use uuid::Uuid;

fn make_actor(username: &str, display_name: &str) -> ActorSummary {
    ActorSummary {
        user_id: Uuid::new_v4(),
        username: username.to_string(),
        display_name: display_name.to_string(),
        avatar_url: None,
    }
}

#[test]
fn coalesced_actor_names_handles_zero_one_two_and_many_actors() {
    let (label0, preview0) = format_coalesced_actor_names(&[]);
    assert_eq!(label0, "Barista Coffee Labs");
    assert!(preview0.is_empty());

    let a1 = make_actor("ryan", "Ryan Wibawa");
    let (label1, preview1) = format_coalesced_actor_names(std::slice::from_ref(&a1));
    assert_eq!(label1, "Ryan Wibawa");
    assert_eq!(preview1.len(), 1);

    let a2 = make_actor("bayu", "Bayu Prawiro");
    let (label2, preview2) = format_coalesced_actor_names(&[a1.clone(), a2.clone()]);
    assert_eq!(label2, "Ryan Wibawa dan Bayu Prawiro");
    assert_eq!(preview2.len(), 2);

    let a3 = make_actor("sherry", "Sherry Hsu");
    let a4 = make_actor("tetsu", "Tetsu Kasuya");
    let (label4, preview4) =
        format_coalesced_actor_names(&[a1, a2, a3, a4]);
    assert_eq!(label4, "Ryan Wibawa, Bayu Prawiro, dan 2 lainnya");
    assert_eq!(preview4.len(), 3);
}

#[test]
fn full_jitter_delay_bounds_and_retry_after_and_rmax() {
    assert_eq!(R_MAX, 5);
    assert_eq!(T_BASE_MS, 1_000);
    assert_eq!(T_MAX_MS, 60_000);

    for attempt in 0..8 {
        let cap = T_BASE_MS.saturating_mul(1_u64 << attempt.min(6)).min(T_MAX_MS);
        let delay = compute_full_jitter_delay_ms(attempt, None);
        assert!(delay <= cap, "delay {delay} exceeded cap {cap}");
    }

    let retry_respected = compute_full_jitter_delay_ms(0, Some(15));
    assert!((15_000..=60_000).contains(&retry_respected));

    let retry_clamped = compute_full_jitter_delay_ms(1, Some(120));
    assert_eq!(retry_clamped, 60_000);
}

#[test]
fn quiet_hours_evaluates_same_day_cross_midnight_timezones_and_disabled() {
    let uid = Uuid::new_v4();
    let mut pref = NotificationPreferenceRow::default_for(uid);
    let utc_1600 = Utc.with_ymd_and_hms(2026, 11, 13, 16, 0, 0).unwrap();

    // Disabled when None
    assert!(!is_quiet_hours_now(&pref, utc_1600));

    // Same-day window: 13:00-15:00 WIB (780..900), UTC 07:00 == 14:00 WIB -> inside
    pref.quiet_hours_start_minute = Some(13 * 60);
    pref.quiet_hours_end_minute = Some(15 * 60);
    let utc_0700 = Utc.with_ymd_and_hms(2026, 11, 13, 7, 0, 0).unwrap();
    assert!(is_quiet_hours_now(&pref, utc_0700));
    assert!(!is_quiet_hours_now(&pref, utc_1600));

    // Cross-midnight window: 22:00-07:00 (1320..420)
    pref.quiet_hours_start_minute = Some(22 * 60);
    pref.quiet_hours_end_minute = Some(7 * 60);

    // 16:00 UTC -> 23:00 WIB -> inside; 06:30 UTC -> 13:30 WIB -> outside
    pref.timezone = "Asia/Jakarta".to_string();
    assert!(is_quiet_hours_now(&pref, utc_1600));
    let utc_0630 = Utc.with_ymd_and_hms(2026, 11, 13, 6, 30, 0).unwrap();
    assert!(!is_quiet_hours_now(&pref, utc_0630));

    // 23:30 UTC -> 07:30 WITA (+8 -> outside!), 08:30 WIT (+9 -> outside!), 23:30 UTC (UTC -> inside!)
    let utc_2330 = Utc.with_ymd_and_hms(2026, 11, 13, 23, 30, 0).unwrap();
    pref.timezone = "Asia/Makassar".to_string();
    assert!(!is_quiet_hours_now(&pref, utc_2330));
    pref.timezone = "Asia/Jayapura".to_string();
    assert!(!is_quiet_hours_now(&pref, utc_2330));
    pref.timezone = "UTC".to_string();
    assert!(is_quiet_hours_now(&pref, utc_2330));

    // Start == End disables quiet hours
    pref.quiet_hours_start_minute = Some(600);
    pref.quiet_hours_end_minute = Some(600);
    assert!(!is_quiet_hours_now(&pref, utc_1600));

    // Category filter check
    pref.enable_likes_and_recipe_tries = false;
    assert!(!is_event_enabled_for_user(
        &pref,
        NotificationCategory::ThreadActivity,
        "THREAD_LIKE_COALESCED"
    ));
    assert!(is_event_enabled_for_user(
        &pref,
        NotificationCategory::ThreadActivity,
        "THREAD_REPLY_OR_QUOTE"
    ));
}

#[test]
fn notification_cursor_roundtrips_and_rejects_malformed_tokens() {
    let ts = Utc::now();
    let id = Uuid::now_v7();
    let encoded = NotificationCursor { updated_at: ts, id }.encode();
    let decoded = NotificationCursor::decode(&encoded).expect("cursor valid");
    assert_eq!(decoded.id, id);
    assert_eq!(decoded.updated_at.timestamp_micros(), ts.timestamp_micros());

    assert!(NotificationCursor::decode("not*valid*base64!").is_err());
    assert!(NotificationCursor::decode("YWJjZGVm").is_err()); // "abcdef" (no '|')
    assert!(NotificationCursor::decode("YWJjfDEyMw").is_err()); // "abc|123"
    assert!(NotificationCursor::decode("MTIzNDV8bm90LXV1aWQ").is_err()); // "12345|not-uuid"
}

#[test]
fn event_builders_and_fcm_v1_payload_contract() {
    let target = Uuid::new_v4();
    let conv_id = Uuid::new_v4();
    let sender = make_actor("barista1", "Ardi Brews");

    let chat_evt = build_chat_message_event(target, conv_id, sender.clone(), "Halo V60!");
    assert_eq!(chat_evt.category, NotificationCategory::Chat);
    assert_eq!(chat_evt.action_buttons.len(), 2);
    assert_eq!(chat_evt.action_buttons[0], NotificationActionButton::InlineReply);

    let fcm_json = build_fcm_v1_message("fcm_tok_abc", &chat_evt);
    assert_eq!(fcm_json["message"]["token"], "fcm_tok_abc");
    assert_eq!(fcm_json["message"]["android"]["priority"], "HIGH");
    assert_eq!(
        fcm_json["message"]["android"]["notification"]["channel_id"],
        "social_coffee_chat"
    );
    assert_eq!(
        fcm_json["message"]["android"]["collapse_key"],
        format!("CHAT_ROOM:{conv_id}")
    );
    assert!(fcm_json["message"]["data"]["action_buttons_json"]
        .as_str()
        .unwrap()
        .contains("INLINE_REPLY"));

    let follow_evt = build_new_follower_event(target, sender.clone(), true);
    assert_eq!(follow_evt.action_buttons[0], NotificationActionButton::QuickFollback);

    let mutual_evt = build_mutual_follback_event(target, sender.clone());
    assert_eq!(mutual_evt.event_type, "SOCIAL_MUTUAL_FOLLBACK_ACCEPTED");

    let thread_id = Uuid::new_v4();
    let reply_evt = build_thread_reply_event(target, thread_id, sender.clone(), "Mantap!");
    assert_eq!(reply_evt.deep_link_path, format!("/social/threads/{thread_id}"));

    let like_evt = build_thread_like_coalesce_seed(target, thread_id, sender);
    assert!(!like_evt.priority_high);
    assert!(like_evt.coalesce_key.as_deref().unwrap().starts_with("LIKE:thread:"));

    let tier_evt = build_tier_upgrade_event(target, "Pro");
    assert!(tier_evt.is_critical_transactional);
    assert_eq!(tier_evt.category, NotificationCategory::Billing);

    let streak_evt = build_streak_milestone_event(target, 7, 120);
    assert_eq!(streak_evt.category, NotificationCategory::Gamification);
    assert_eq!(streak_evt.deep_link_path, "/brew/timer");
}

#[tokio::test]
async fn noop_producer_and_unconfigured_fcm_gateway_dry_run_and_ws_frames() {
    assert_eq!(NOTIF_STREAM_KEY, "notif:stream:instant");
    assert_eq!(NOTIF_DLQ_KEY, "notif:stream:dlq");
    assert_eq!(coffe_labs_backend::core::redis::REDIS_STREAM_MAXLEN, 1_000);
    assert_eq!(coffe_labs_backend::core::redis::REDIS_POOL_SIZE, 4);

    let producer = NotificationProducerService::noop_for_tests();
    let sender = make_actor("kopibara", "KopiBara Bot");
    let evt = build_tier_upgrade_event(Uuid::new_v4(), "Business");
    producer.publish(evt.clone());
    producer.publish_coalesced(build_thread_like_coalesce_seed(
        Uuid::new_v4(),
        Uuid::new_v4(),
        sender,
    ));

    let gateway = FcmV1Gateway::from_env();
    if !gateway.is_configured() {
        let outcome = gateway.send_to_token("tok_dummy", &evt).await;
        assert_eq!(outcome, FcmDeliveryOutcome::UnconfiguredDryRun);
    }

    let notif_id = Uuid::now_v7();
    let actor_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let server_frame = ServerFrame::PushNotificationEvent {
        notification_id: notif_id,
        category: "SOCIAL_GRAPH".to_string(),
        event_type: "SOCIAL_NEW_FOLLOWER".to_string(),
        title: "Ardi mengikuti Anda".to_string(),
        body: "Follback sekarang!".to_string(),
        deep_link_path: "/social/profile/@ardi".to_string(),
        collapse_key: None,
        entity_id: Some(actor_id.to_string()),
        actor_id: Some(actor_id),
        conversation_id: None,
        action_buttons: vec!["QUICK_FOLLBACK".to_string(), "MARK_AS_READ".to_string()],
        unread_count: 3,
        created_at: Utc::now(),
    };
    let wire = serde_json::to_string(&server_frame).expect("serialize push frame");
    assert!(wire.contains("\"type\":\"push_notification_event\""));
    let decoded: ServerFrame = serde_json::from_str(&wire).expect("deserialize push frame");
    match decoded {
        ServerFrame::PushNotificationEvent {
            notification_id,
            unread_count,
            action_buttons,
            ..
        } => {
            assert_eq!(notification_id, notif_id);
            assert_eq!(unread_count, 3);
            assert_eq!(action_buttons[0], "QUICK_FOLLBACK");
        }
        other => panic!("unexpected variant: {other:?}"),
    }

    let client_frame = ClientFrame::SetActiveRoom {
        conversation_id: Some(room_id),
    };
    let client_wire = serde_json::to_string(&client_frame).expect("serialize set_active_room");
    assert!(client_wire.contains("\"type\":\"set_active_room\""));
    let back_client: ClientFrame =
        serde_json::from_str(&client_wire).expect("deserialize set_active_room");
    assert!(matches!(
        back_client,
        ClientFrame::SetActiveRoom {
            conversation_id: Some(id)
        } if id == room_id
    ));
}
