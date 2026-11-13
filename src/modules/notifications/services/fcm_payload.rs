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

    let android_priority = if event.priority_high { "HIGH" } else { "NORMAL" };
    let apns_priority = if event.priority_high { "10" } else { "5" };
    let apns_category = match event.category {
        NotificationCategory::Chat => "SOCIAL_CHAT",
        NotificationCategory::SocialGraph => "SOCIAL_GRAPH",
        _ => "SOCIAL_GENERIC",
    };

    let mut android = json!({
        "priority": android_priority,
        "ttl": format!("{}s", event.ttl_secs),
        "notification": {
            "channel_id": event.category.android_channel_id(),
            "click_action": "FLUTTER_NOTIFICATION_CLICK",
            "title": event.title,
            "body": event.body,
        }
    });
    if let Some(ck) = &event.collapse_key {
        android["collapse_key"] = json!(ck);
        android["notification"]["tag"] = json!(ck);
    }

    let mut apns_headers = json!({
        "apns-priority": apns_priority,
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
                        "category": apns_category,
                        "thread-id": event.collapse_key.clone().unwrap_or_else(|| event.category.as_str().to_string()),
                    }
                }
            }
        }
    })
}
