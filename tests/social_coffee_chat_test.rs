use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use coffe_labs_backend::modules::chat::{
    error::ChatError,
    protocol::{
        ChatMessageDto, ClientFrame, ConversationCursorDto, InteractiveAttachment, MessageType,
        ReceiptStatus, ServerFrame,
    },
    repositories::conversation_repo::compute_direct_pair_key,
    services::{
        connection_hub::ConnectionHub,
        moderation_guard::{ModerationGuard, ModerationMode, SocketRateLimiter},
    },
};

#[test]
fn direct_pair_key_is_canonical_regardless_of_order() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    assert_eq!(compute_direct_pair_key(a, b), compute_direct_pair_key(b, a));
}

#[test]
fn client_and_server_frames_roundtrip_json_and_msgpack() {
    let conv_id = Uuid::new_v4();
    let client_msg_id = Uuid::new_v4();

    let send_frame = ClientFrame::SendMessage {
        client_msg_id,
        conversation_id: conv_id,
        reply_to_msg_id: None,
        message_type: MessageType::RecipeCard,
        body_text: Some("Cobain resep V60 favoritku!".to_string()),
        attachment: Some(InteractiveAttachment::V60RecipeCard {
            recipe_id: "42".to_string(),
            title: "Dual-Temp V60 - Sherry Hsu".to_string(),
            brewer_name: "V60 Dripper".to_string(),
            dose_grams: 15.0,
            water_ml: 240.0,
            temp_c: 92,
            target_duration_sec: 150,
            steps_preview: vec!["Bloom 45ml".to_string()],
        }),
    };

    let json = serde_json::to_string(&send_frame).expect("json encode");
    assert!(json.contains("\"type\":\"send_message\""));
    assert!(json.contains("\"kind\":\"v60_recipe_card\""));

    let decoded_json: ClientFrame = serde_json::from_str(&json).expect("json decode");
    match decoded_json {
        ClientFrame::SendMessage {
            client_msg_id: d_id,
            message_type,
            attachment: Some(InteractiveAttachment::V60RecipeCard { recipe_id, .. }),
            ..
        } => {
            assert_eq!(d_id, client_msg_id);
            assert_eq!(message_type, MessageType::RecipeCard);
            assert_eq!(recipe_id, "42");
        }
        _ => panic!("Unexpected decoded frame"),
    }

    let sync_req = ClientFrame::SyncDelta {
        cursors: vec![ConversationCursorDto {
            conversation_id: conv_id,
            local_max_seq_id: 18,
        }],
    };
    let bin = rmp_serde::to_vec_named(&sync_req).expect("msgpack encode");
    let decoded_bin: ClientFrame = rmp_serde::from_slice(&bin).expect("msgpack decode");
    assert!(matches!(decoded_bin, ClientFrame::SyncDelta { .. }));

    let ack = ServerFrame::ReceiptWatermarkUpdate {
        conversation_id: conv_id,
        user_id: Uuid::new_v4(),
        status: ReceiptStatus::Read,
        up_to_seq_id: 99,
        updated_at: Utc::now(),
    };
    let ack_json = serde_json::to_string(&ack).expect("ack json");
    assert!(ack_json.contains("\"type\":\"receipt_watermark_update\""));

    let _msg_dto = ChatMessageDto {
        id: Uuid::now_v7(),
        conversation_id: conv_id,
        seq_id: 1,
        sender_id: Uuid::new_v4(),
        client_msg_id,
        reply_to_msg_id: None,
        message_type: MessageType::BeanCard,
        body_text: None,
        attachment: Some(InteractiveAttachment::CoffeeBeanCard {
            bean_id: None,
            roastery: "Fugol Coffee Roasters".to_string(),
            bean_name: "Halu Banana Natural".to_string(),
            origin: "Gunung Halu, Jawa Barat".to_string(),
            process: "Natural".to_string(),
            tasting_notes: vec!["Banana".to_string(), "Sweet Caramel".to_string()],
            image_url: None,
        }),
        is_edited: false,
        is_deleted: false,
        created_at: Utc::now(),
    };
}

#[test]
fn connection_hub_supports_multi_device_fanout() {
    let hub = ConnectionHub::new();
    let user_id = Uuid::new_v4();

    let (s1, mut rx1, online1) = hub.register(user_id);
    let (s2, mut rx2, online2) = hub.register(user_id);
    assert!(online1);
    assert!(!online2);
    assert!(hub.is_local_online(user_id));

    let delivered = hub.send_local(user_id, Arc::new(ServerFrame::Pong { timestamp_ms: 12345 }));
    assert_eq!(delivered, 2);
    assert!(rx1.try_recv().is_ok());
    assert!(rx2.try_recv().is_ok());

    assert!(!hub.unregister(user_id, s1));
    assert!(hub.is_local_online(user_id));
    assert!(hub.unregister(user_id, s2));
    assert!(!hub.is_local_online(user_id));
}

#[test]
fn moderation_guard_filters_profanity_and_enforces_quotas() {
    let guard = ModerationGuard::new();

    let clean = guard
        .check_text("Seduh V60 pagi ini enak banget!", ModerationMode::MaskWithAsterisks)
        .expect("clean text");
    assert_eq!(clean, "Seduh V60 pagi ini enak banget!");

    let masked = guard
        .check_text("dasar anjing spam slot gacor", ModerationMode::MaskWithAsterisks)
        .expect("masked");
    assert!(!masked.contains("anjing"));
    assert!(masked.contains("***"));

    let rejected = guard.check_text("slot gacor maxwin88", ModerationMode::RejectMessage);
    assert!(matches!(rejected, Err(ChatError::ProfanityRejected(_))));

    let limiter = SocketRateLimiter::new();
    for _ in 0..8 {
        assert!(limiter.check_message_quota().is_ok());
    }
    assert!(matches!(
        limiter.check_message_quota(),
        Err(ChatError::RateLimited(_))
    ));

    assert!(limiter.check_typing_quota().is_ok());
    assert!(limiter.check_typing_quota().is_ok());
    assert!(matches!(
        limiter.check_typing_quota(),
        Err(ChatError::RateLimited(_))
    ));
}
