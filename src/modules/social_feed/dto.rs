use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::modules::social_feed::request_dto::{
    CreateThreadRequest, DirectorySearchQuery, ProfileTabFeedQuery, TimelineFeedQuery,
    UpdateMyProfileRequest,
};

use crate::modules::social_feed::attachments::{
    BadgeMiniDto, MediaItemDto, RecentBrewSnapshot, ThreadAttachmentPayload,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileFeedTab {
    AllThreads,
    BrewLogs,
    CreatedRecipes,
    Media,
}

impl ProfileFeedTab {
    pub fn from_query(raw: Option<&str>) -> Self {
        match raw.unwrap_or("ALL_THREADS").trim().to_uppercase().as_str() {
            "BREW_LOGS" => Self::BrewLogs,
            "CREATED_RECIPES" => Self::CreatedRecipes,
            "MEDIA" => Self::Media,
            _ => Self::AllThreads,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializedProfileDto {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub account_tier: String,
    pub total_xp: i64,
    pub barista_level: i32,
    pub barista_rank_title: String,
    pub level_progress_ratio: f32,
    pub next_level_xp: i64,
    pub current_streak_days: i32,
    pub unlocked_badge_ids: Vec<u32>,
    pub top_featured_badges: Vec<BadgeMiniDto>,
    pub followers_count: i32,
    pub following_count: i32,
    pub total_threads_count: i32,
    pub total_brews_count: i32,
    pub total_recipes_created_count: i32,
    pub total_recipe_tries_received: i32,
    pub recent_brew_snapshot: Option<RecentBrewSnapshot>,
    pub is_followed_by_viewer: bool,
    pub is_following_viewer: bool,
    pub is_own_profile: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDirectoryEntryDto {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub account_tier: String,
    pub total_xp: i64,
    pub barista_level: i32,
    pub barista_rank_title: String,
    pub followers_count: i32,
    pub total_threads_count: i32,
    pub recent_brew_snapshot: Option<RecentBrewSnapshot>,
    pub is_followed_by_viewer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotedThreadPreviewDto {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub body_text: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialThreadItemDto {
    pub id: Uuid,
    pub author_id: Uuid,
    pub author_username: String,
    pub author_display_name: String,
    pub author_avatar_url: Option<String>,
    pub author_tier: String,
    pub author_barista_level: i32,
    pub author_barista_rank_title: String,
    pub kind: String,
    pub parent_thread_id: Option<Uuid>,
    pub root_thread_id: Option<Uuid>,
    pub quoted_thread_id: Option<Uuid>,
    pub quoted_preview: Option<QuotedThreadPreviewDto>,
    pub depth_level: i16,
    pub body_text: String,
    pub has_media: bool,
    pub media_urls: Vec<MediaItemDto>,
    pub attachment: Option<ThreadAttachmentPayload>,
    pub likes_count: i32,
    pub replies_count: i32,
    pub reposts_count: i32,
    pub try_recipe_count: i32,
    pub is_pinned_on_profile: bool,
    pub is_liked_by_me: bool,
    pub is_reposted_by_me: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadReplyNodeDto {
    pub thread: SocialThreadItemDto,
    pub children: Vec<ThreadReplyNodeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadDetailResponseDto {
    pub root: SocialThreadItemDto,
    pub replies: Vec<ThreadReplyNodeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginatedFeedDto<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowToggleResponseDto {
    pub target_user_id: Uuid,
    pub is_followed_by_viewer: bool,
    pub followers_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikeToggleResponseDto {
    pub thread_id: Uuid,
    pub is_liked_by_me: bool,
    pub likes_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepostToggleResponseDto {
    pub thread_id: Uuid,
    pub is_reposted_by_me: bool,
    pub reposts_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryRecipeResponseDto {
    pub thread_id: Uuid,
    pub try_recipe_count: i32,
    pub author_awarded_xp: i64,
    pub author_new_total_xp: i64,
    pub author_new_level: i32,
}
