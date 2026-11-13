use serde::Deserialize;
use uuid::Uuid;

use crate::modules::social_feed::attachments::{
    MediaItemDto, RecentBrewSnapshot, ThreadAttachmentPayload,
};

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileTabFeedQuery {
    pub tab: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimelineFeedQuery {
    pub scope: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DirectorySearchQuery {
    pub q: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateThreadRequest {
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub body_text: String,
    #[serde(default)]
    pub parent_thread_id: Option<Uuid>,
    #[serde(default)]
    pub quoted_thread_id: Option<Uuid>,
    #[serde(default)]
    pub media_urls: Vec<MediaItemDto>,
    #[serde(default)]
    pub attachment: Option<ThreadAttachmentPayload>,
    #[serde(default)]
    pub recent_brew_snapshot: Option<RecentBrewSnapshot>,
    #[serde(default)]
    pub pin_on_profile: bool,
}

fn default_kind() -> String {
    "STANDARD_POST".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMyProfileRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub recent_brew_snapshot: Option<RecentBrewSnapshot>,
}
