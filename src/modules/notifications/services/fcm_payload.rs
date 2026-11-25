use std::collections::HashMap;

use serde_json::{json, Value};

use crate::modules::notifications::events::{NotificationCategory, OmnichannelNotifEvent};

pub fn build_fcm_v1_message(device_token: &str, event: &OmnichannelNotifEvent) -> Value {
    let mut data: HashMap<&str, String> = HashMap::new();
    data.insert("event_type", event.event_type.clone());
    data.insert("category", event.category.as_str().to_string());
    data.insert("deep_link_path", event.deep_link_path.clone());
    data.insert("notification_id", event.event_id.to_string());
    data.insert("title", event.title.clone());
    data.insert("body", event.body.clone());

    if let Some(eid) = &event.entity_id {
        data.insert("entity_id", eid.clone());
    }
    if let Some(actor) = &event.actor {
        data.insert("actor_id", actor.user_id.to_string());
        let sender_name = if actor.display_name.trim().is_empty() {
            actor.username.trim().to_string()
        } else {
            actor.display_name.trim().to_string()
        };
        if !sender_name.is_empty() {
            data.insert("sender_name", sender_name);
        }
        if let Some(avatar) = actor.avatar_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            data.insert("sender_avatar_url", avatar.to_string());
        }
    }
    if let Some(cid) = event.conversation_id {
        data.insert("conversation_id", cid.to_string());
    }
    if let Some(ck) = &event.collapse_key {
        data.insert("collapse_key", ck.clone());
    }
    let btns: Vec<&str> = event.action_buttons.iter().map(|b| b.wire_str()).collect();
    if let Ok(encoded) = serde_json::to_string(&btns) {
        data.insert("action_buttons_json", encoded);
    }

    let is_chat = event.category == NotificationCategory::Chat;
    let android_priority = if event.priority_high || is_chat { "HIGH" } else { "NORMAL" };
    let notif_priority = if is_chat {
        "PRIORITY_MAX"
    } else if event.priority_high {
        "PRIORITY_HIGH"
    } else {
        "PRIORITY_DEFAULT"
    };
    let apns_priority = if event.priority_high || is_chat { "10" } else { "5" };
    let apns_category = match event.category {
        NotificationCategory::Chat => "SOCIAL_CHAT",
        NotificationCategory::SocialGraph => "SOCIAL_GRAPH",
        _ => "SOCIAL_GENERIC",
    };

    let mut android = json!({
        "priority": android_priority,
        "direct_boot_ok": true,
        "ttl": format!("{}s", event.ttl_secs),
        "notification": {
            "channel_id": event.category.android_channel_id(),
            "click_action": "FLUTTER_NOTIFICATION_CLICK",
            "title": event.title,
            "body": event.body,
            "sound": "default",
            "default_sound": true,
            "default_vibrate_timings": true,
            "notification_priority": notif_priority,
            "visibility": "PUBLIC",
            "notification_count": 1
        }
    });
    if let Some(ck) = &event.collapse_key {
        android["collapse_key"] = json!(ck);
        android["notification"]["tag"] = json!(ck);
    }

    let mut apns_headers = json!({
        "apns-priority": apns_priority,
        "apns-push-type": "alert",
    });
    if let Some(ck) = &event.collapse_key {
        apns_headers["apns-collapse-id"] = json!(ck);
    }

    json!({
        "message": {
            "token": device_token,
            "notification": {
                "title": event.title,
                "body": event.body,
            },
            "data": data,
            "android": android,
            "apns": {
                "headers": apns_headers,
                "payload": {
                    "aps": {
                        "sound": "default",
                        "mutable-content": 1,
                        "interruption-level": if is_chat { "time-sensitive" } else { "active" },
                        "category": apns_category,
                        "thread-id": event.collapse_key.clone().unwrap_or_else(|| event.category.as_str().to_string()),
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::notifications::{
        builders::build_chat_message_event,
        events::ActorSummary,
    };
    use uuid::Uuid;

    #[test]
    fn chat_fcm_payload_enforces_whatsapp_style_android_heads_up() {
        let sender = ActorSummary {
            user_id: Uuid::now_v7(),
            username: "rusdifn08".into(),
            display_name: "Rusdi Barista".into(),
            avatar_url: Some("https://cdn.example.com/a.jpg".into()),
        };
        let evt = build_chat_message_event(Uuid::now_v7(), Uuid::now_v7(), sender, "Halo, coba V60 baru!");
        let msg = build_fcm_v1_message("fcm_tok_123", &evt);

        assert_eq!(msg["message"]["android"]["priority"], "HIGH");
        assert_eq!(msg["message"]["android"]["notification"]["channel_id"], "social_coffee_chat_v2");
        assert_eq!(msg["message"]["android"]["notification"]["notification_priority"], "PRIORITY_MAX");
        assert_eq!(msg["message"]["android"]["notification"]["default_sound"], true);
        assert_eq!(msg["message"]["android"]["notification"]["default_vibrate_timings"], true);
        assert_eq!(msg["message"]["android"]["notification"]["visibility"], "PUBLIC");
        assert_eq!(msg["message"]["data"]["sender_name"], "Rusdi Barista");
    }
}
