use chrono::Utc;
use uuid::Uuid;

use super::events::{ActorSummary, NotificationActionButton, NotificationCategory, OmnichannelNotifEvent};

fn actor_label(actor: &ActorSummary) -> String {
    let trimmed = actor.display_name.trim();
    if trimmed.is_empty() { format!("@{}", actor.username) } else { trimmed.to_string() }
}

fn base_event(
    target_user_id: Uuid,
    category: NotificationCategory,
    event_type: &str,
    title: String,
    body: String,
    deep_link_path: String,
) -> OmnichannelNotifEvent {
    let now = Utc::now();
    OmnichannelNotifEvent {
        event_id: Uuid::now_v7(),
        target_user_id,
        category,
        event_type: event_type.to_string(),
        title,
        body,
        deep_link_path,
        coalesce_key: None,
        collapse_key: None,
        priority_high: true,
        ttl_secs: category.default_ttl_secs(),
        created_at: now,
        actor: None,
        actors_preview: Vec::new(),
        aggregated_count: 1,
        entity_id: None,
        conversation_id: None,
        action_buttons: vec![NotificationActionButton::MarkAsRead],
        is_critical_transactional: false,
        attempt: 0,
        first_enqueued_at: now,
    }
}

fn actor_event(
    target_user_id: Uuid,
    category: NotificationCategory,
    event_type: &str,
    title: String,
    body: String,
    deep_link_path: String,
    actor: ActorSummary,
    entity_id: String,
) -> OmnichannelNotifEvent {
    let mut evt = base_event(target_user_id, category, event_type, title, body, deep_link_path);
    evt.entity_id = Some(entity_id);
    evt.actors_preview = vec![actor.clone()];
    evt.actor = Some(actor);
    evt
}

pub fn build_chat_message_event(
    target_user_id: Uuid,
    conversation_id: Uuid,
    sender: ActorSummary,
    preview_text: &str,
) -> OmnichannelNotifEvent {
    let title = if sender.display_name.trim().is_empty() {
        format!("Pesan dari @{}", sender.username)
    } else {
        sender.display_name.trim().to_string()
    };
    let mut evt = actor_event(
        target_user_id,
        NotificationCategory::Chat,
        "DIRECT_CHAT_MESSAGE_RECEIVED",
        title,
        preview_text.trim().to_string(),
        format!("/social/chat/room/{conversation_id}"),
        sender,
        conversation_id.to_string(),
    );
    evt.collapse_key = Some(format!("CHAT_ROOM:{conversation_id}"));
    evt.conversation_id = Some(conversation_id);
    evt.action_buttons = vec![NotificationActionButton::InlineReply, NotificationActionButton::MarkAsRead];
    evt
}

pub fn build_new_follower_event(
    target_user_id: Uuid,
    actor: ActorSummary,
    is_follback_opportunity: bool,
) -> OmnichannelNotifEvent {
    let name = actor_label(&actor);
    let body = if is_follback_opportunity {
        format!("{name} mulai mengikuti Anda. Follback untuk langsung mengobrol!")
    } else {
        format!("{name} mulai mengikuti perjalanan seduh Anda.")
    };
    let (entity_id, link) = (actor.user_id.to_string(), format!("/social/profile/@{}", actor.username));
    let mut evt = actor_event(
        target_user_id,
        NotificationCategory::SocialGraph,
        "SOCIAL_NEW_FOLLOWER",
        format!("{name} mengikuti Anda"),
        body,
        link,
        actor,
        entity_id,
    );
    if is_follback_opportunity {
        evt.action_buttons = vec![NotificationActionButton::QuickFollback, NotificationActionButton::MarkAsRead];
    }
    evt
}

pub fn build_mutual_follback_event(target_user_id: Uuid, actor: ActorSummary) -> OmnichannelNotifEvent {
    let name = actor_label(&actor);
    let (entity_id, link) = (actor.user_id.to_string(), format!("/social/profile/@{}", actor.username));
    actor_event(
        target_user_id,
        NotificationCategory::SocialGraph,
        "SOCIAL_MUTUAL_FOLLBACK_ACCEPTED",
        format!("Mutual Baru dengan {name}!"),
        "Kalian sekarang saling mengikuti. Mulai bagikan resep V60 via chat!".to_string(),
        link,
        actor,
        entity_id,
    )
}

pub fn build_thread_reply_event(
    target_user_id: Uuid,
    thread_id: Uuid,
    actor: ActorSummary,
    reply_preview: &str,
) -> OmnichannelNotifEvent {
    let name = actor_label(&actor);
    let mut evt = actor_event(
        target_user_id,
        NotificationCategory::ThreadActivity,
        "THREAD_REPLY_OR_QUOTE",
        format!("{name} membalas thread Anda"),
        reply_preview.trim().to_string(),
        format!("/social/threads/{thread_id}"),
        actor,
        thread_id.to_string(),
    );
    evt.collapse_key = Some(format!("THREAD:{thread_id}"));
    evt
}

pub fn build_thread_like_coalesce_seed(
    target_user_id: Uuid,
    thread_id: Uuid,
    actor: ActorSummary,
) -> OmnichannelNotifEvent {
    let name = actor_label(&actor);
    let mut evt = actor_event(
        target_user_id,
        NotificationCategory::ThreadActivity,
        "THREAD_LIKE_COALESCED",
        format!("{name} menyukai catatan seduh Anda"),
        "Resep dan diskusi seduh Anda mendapat apresiasi dari komunitas Coffee Labs.".to_string(),
        format!("/social/threads/{thread_id}"),
        actor,
        thread_id.to_string(),
    );
    evt.priority_high = false;
    evt.coalesce_key = Some(format!("LIKE:thread:{thread_id}:target:{target_user_id}"));
    evt.collapse_key = Some(format!("THREAD_LIKE:{thread_id}"));
    evt
}

pub fn build_tier_upgrade_event(target_user_id: Uuid, new_tier_label: &str) -> OmnichannelNotifEvent {
    let mut evt = base_event(
        target_user_id, NotificationCategory::Billing, "TRANSACTIONAL_TIER_UPDATED",
        format!("Paket Coffee Labs {new_tier_label} Aktif!"),
        format!("Akun Anda kini berada di tier {new_tier_label}. Nikmati seluruh fitur Pro tanpa batas."),
        "/settings/subscription".to_string(),
    );
    evt.is_critical_transactional = true;
    evt
}

pub fn build_streak_milestone_event(target_user_id: Uuid, streak_days: i32, xp_awarded: i32) -> OmnichannelNotifEvent {
    let mut evt = base_event(
        target_user_id, NotificationCategory::Gamification, "LIVE_BREW_STEP_OR_STREAK_REMINDER",
        format!("Streak Seduh {streak_days} Hari Tercapai!"),
        format!("Luar biasa! Anda baru saja meraih +{xp_awarded} XP seduh spesialti hari ini."),
        "/brew/timer".to_string(),
    );
    evt.priority_high = false;
    evt
}
