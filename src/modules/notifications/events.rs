use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationCategory {
    Chat,
    SocialGraph,
    ThreadActivity,
    Gamification,
    Billing,
}

impl NotificationCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "CHAT",
            Self::SocialGraph => "SOCIAL_GRAPH",
            Self::ThreadActivity => "THREAD_ACTIVITY",
            Self::Gamification => "GAMIFICATION",
            Self::Billing => "BILLING",
        }
    }

    pub fn parse_lossy(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "CHAT" => Some(Self::Chat),
            "SOCIAL_GRAPH" => Some(Self::SocialGraph),
            "THREAD_ACTIVITY" => Some(Self::ThreadActivity),
            "GAMIFICATION" => Some(Self::Gamification),
            "BILLING" => Some(Self::Billing),
            _ => None,
        }
    }

    pub fn default_ttl_secs(self) -> u32 {
        match self {
            Self::Chat | Self::Gamification => 3_600,
            Self::SocialGraph | Self::ThreadActivity => 86_400,
            Self::Billing => 604_800,
        }
    }

    pub fn android_channel_id(self) -> &'static str {
        match self {
            Self::Chat => "social_coffee_chat",
            Self::SocialGraph => "social_coffee_graph",
            Self::ThreadActivity => "social_coffee_threads",
            Self::Gamification | Self::Billing => "social_coffee_billing",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActorSummary {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationActionButton {
    QuickFollback,
    InlineReply,
    MarkAsRead,
}

impl NotificationActionButton {
    pub fn wire_str(self) -> &'static str {
        match self {
            Self::QuickFollback => "QUICK_FOLLBACK",
            Self::InlineReply => "INLINE_REPLY",
            Self::MarkAsRead => "MARK_AS_READ",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnichannelNotifEvent {
    pub event_id: Uuid,
    pub target_user_id: Uuid,
    pub category: NotificationCategory,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub deep_link_path: String,
    pub coalesce_key: Option<String>,
    pub collapse_key: Option<String>,
    pub priority_high: bool,
    pub ttl_secs: u32,
    pub created_at: DateTime<Utc>,
    pub actor: Option<ActorSummary>,
    pub actors_preview: Vec<ActorSummary>,
    pub aggregated_count: i32,
    pub entity_id: Option<String>,
    pub conversation_id: Option<Uuid>,
    pub action_buttons: Vec<NotificationActionButton>,
    pub is_critical_transactional: bool,
    pub attempt: u32,
    pub first_enqueued_at: DateTime<Utc>,
}

/// Menghasilkan label aktor koalesensi yang alami untuk N=1, N=2, maupun N>2 aktor.
pub fn format_coalesced_actor_names(actors: &[ActorSummary]) -> (String, Vec<ActorSummary>) {
    let preview: Vec<ActorSummary> = actors.iter().take(3).cloned().collect();
    let names: Vec<&str> = actors
        .iter()
        .map(|a| {
            let trimmed = a.display_name.trim();
            if trimmed.is_empty() {
                a.username.trim()
            } else {
                trimmed
            }
        })
        .filter(|s| !s.is_empty())
        .collect();

    let n = actors.len();
    let formatted = match (n, names.as_slice()) {
        (0, _) => "Barista Coffee Labs".to_string(),
        (1, [first]) => (*first).to_string(),
        (2, [first, second, ..]) => format!("{first} dan {second}"),
        (_, [first, second, ..]) => {
            let others = n.saturating_sub(2);
            format!("{first}, {second}, dan {others} lainnya")
        }
        (_, [only]) => (*only).to_string(),
        _ => format!("{n} barista"),
    };

    (formatted, preview)
}
