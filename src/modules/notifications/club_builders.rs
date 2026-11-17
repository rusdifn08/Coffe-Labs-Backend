use chrono::Utc;
use uuid::Uuid;

use super::events::{ActorSummary, NotificationActionButton, NotificationCategory, OmnichannelNotifEvent};

fn display_label(actor: &ActorSummary) -> String {
    let trimmed = actor.display_name.trim();
    if trimmed.is_empty() {
        format!("@{}", actor.username)
    } else {
        trimmed.to_string()
    }
}

pub fn build_club_invitation_event(
    target_user_id: Uuid,
    community_id: Uuid,
    community_name: &str,
    invitation_id: Uuid,
    invite_message: &str,
    actor: ActorSummary,
) -> OmnichannelNotifEvent {
    let now = Utc::now();
    let actor_name = display_label(&actor);
    let body = if invite_message.trim().is_empty() {
        format!(
            "{actor_name} mengajakmu bergabung ke Coffee Club \"{community_name}\". Terima atau tolak undangan sekarang!"
        )
    } else {
        format!("\"{}\"", invite_message.trim())
    };

    OmnichannelNotifEvent {
        event_id: Uuid::now_v7(),
        target_user_id,
        category: NotificationCategory::SocialGraph,
        event_type: "CLUB_MEMBER_INVITED".to_string(),
        title: format!("{actor_name} mengundangmu ke Coffee Club {community_name}"),
        body,
        deep_link_path: "/social/clubs".to_string(),
        coalesce_key: Some(format!("CLUB_INVITE:{community_id}:target:{target_user_id}")),
        collapse_key: Some(format!("CLUB_INVITE:{community_id}")),
        priority_high: true,
        ttl_secs: NotificationCategory::SocialGraph.default_ttl_secs(),
        created_at: now,
        actor: Some(actor.clone()),
        actors_preview: vec![actor],
        aggregated_count: 1,
        entity_id: Some(invitation_id.to_string()),
        conversation_id: Some(community_id),
        action_buttons: vec![
            NotificationActionButton::AcceptClubInvite,
            NotificationActionButton::DeclineClubInvite,
            NotificationActionButton::MarkAsRead,
        ],
        is_critical_transactional: false,
        attempt: 0,
        first_enqueued_at: now,
    }
}

pub fn build_club_invite_response_event(
    target_user_id: Uuid,
    community_id: Uuid,
    community_name: &str,
    accepted: bool,
    actor: ActorSummary,
) -> OmnichannelNotifEvent {
    let now = Utc::now();
    let actor_name = display_label(&actor);
    let (event_type, title, body) = if accepted {
        (
            "CLUB_INVITE_ACCEPTED",
            format!("{actor_name} bergabung ke {community_name}!"),
            format!("{actor_name} menerima undangan masuk ke Coffee Club {community_name}."),
        )
    } else {
        (
            "CLUB_INVITE_DECLINED",
            format!("{actor_name} menolak undangan {community_name}"),
            format!("{actor_name} memilih tidak bergabung ke Coffee Club {community_name}."),
        )
    };

    OmnichannelNotifEvent {
        event_id: Uuid::now_v7(),
        target_user_id,
        category: NotificationCategory::SocialGraph,
        event_type: event_type.to_string(),
        title,
        body,
        deep_link_path: format!("/social/clubs/{community_id}"),
        coalesce_key: None,
        collapse_key: Some(format!("CLUB_RESP:{community_id}")),
        priority_high: accepted,
        ttl_secs: NotificationCategory::SocialGraph.default_ttl_secs(),
        created_at: now,
        actor: Some(actor.clone()),
        actors_preview: vec![actor],
        aggregated_count: 1,
        entity_id: Some(community_id.to_string()),
        conversation_id: Some(community_id),
        action_buttons: vec![NotificationActionButton::MarkAsRead],
        is_critical_transactional: false,
        attempt: 0,
        first_enqueued_at: now,
    }
}
