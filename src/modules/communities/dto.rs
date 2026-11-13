use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::modules::social_feed::attachments::{
    MediaItemDto, RecentBrewSnapshot, ThreadAttachmentPayload,
};

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateCommunityRequest {
    #[validate(length(min = 3, max = 80, message = "Nama club harus 3-80 karakter."))]
    pub name: String,
    #[serde(default)]
    pub handle_slug: Option<String>,
    #[serde(default)]
    #[validate(length(max = 800, message = "Deskripsi maksimal 800 karakter."))]
    pub description: String,
    #[serde(default = "default_privacy")]
    pub privacy_level: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub banner_url: Option<String>,
    #[serde(default)]
    pub focus_tags: Vec<String>,
}

fn default_privacy() -> String {
    "PUBLIC".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverCommunitiesQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClubFeedQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct JoinBySecretRequest {
    #[validate(length(min = 6, max = 32, message = "Format Secret ID tidak valid."))]
    pub secret_code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateInviteSecretRequest {
    #[serde(default)]
    pub role_granted: Option<String>,
    #[serde(default)]
    pub max_uses: Option<i32>,
    #[serde(default)]
    pub expires_in_hours: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateClubThreadRequest {
    #[serde(default = "default_club_post_kind")]
    pub kind: String,
    #[serde(default)]
    pub body_text: String,
    #[serde(default)]
    pub quoted_thread_id: Option<Uuid>,
    #[serde(default)]
    pub media_urls: Vec<MediaItemDto>,
    #[serde(default)]
    pub attachment: Option<ThreadAttachmentPayload>,
    #[serde(default)]
    pub recent_brew_snapshot: Option<RecentBrewSnapshot>,
    #[serde(default)]
    pub pin_in_club: bool,
}

fn default_club_post_kind() -> String {
    "STANDARD_POST".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunitySummaryDto {
    pub id: Uuid,
    pub handle_slug: String,
    pub name: String,
    pub description: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub privacy_level: String,
    pub owner_id: Uuid,
    pub focus_tags: Vec<String>,
    pub members_count: i32,
    pub threads_count: i32,
    pub shared_recipes_count: i32,
    pub total_club_xp: i64,
    pub viewer_role: Option<String>,
    pub is_member: bool,
    pub match_score_pct: Option<f32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityChannelDto {
    pub id: Uuid,
    pub community_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub linked_conversation_id: Option<Uuid>,
    pub display_order: i16,
    pub is_readonly_announcement: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityWorkspaceDto {
    pub community: CommunitySummaryDto,
    pub channels: Vec<CommunityChannelDto>,
    pub my_role: Option<String>,
    pub is_member: bool,
    pub can_post: bool,
    pub can_manage_invites: bool,
    pub active_secret_code_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityInviteSecretDto {
    pub id: Uuid,
    pub community_id: Uuid,
    pub secret_code_display: String,
    pub invite_url_token: String,
    pub invite_deep_link: String,
    pub invite_web_link: String,
    pub role_granted: String,
    pub max_uses: Option<i32>,
    pub used_count: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinClubResponseDto {
    pub joined: bool,
    pub already_member: bool,
    pub assigned_role: String,
    pub club: CommunitySummaryDto,
    pub default_conversation_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityLeaderboardEntryDto {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub barista_level: i32,
    pub barista_rank_title: String,
    pub role: String,
    pub club_xp_contributed: i64,
    pub brews_shared_count: i32,
    pub joined_at: DateTime<Utc>,
    pub is_followed_by_viewer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverCommunitiesResponseDto {
    pub my_clubs: Vec<CommunitySummaryDto>,
    pub discover_clubs: Vec<CommunitySummaryDto>,
    pub has_more: bool,
}
