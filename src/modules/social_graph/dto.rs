use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::social_graph::models::RelationshipState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniUserAvatarDto {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub barista_rank_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSnapshotDto {
    pub target_user_id: Uuid,
    pub state: RelationshipState,
    pub is_mutual: bool,
    pub is_followed_by_target: bool,
    pub has_pending_incoming_request: bool,
    pub target_is_private: bool,
    pub mutual_followers_count: i64,
    pub mutual_avatars: Vec<MiniUserAvatarDto>,
    pub followers_count: i64,
    pub following_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowMutationResponseDto {
    pub target_user_id: Uuid,
    pub state: RelationshipState,
    pub status: String,
    pub is_idempotent_replay: bool,
    pub target_followers_count: i32,
    pub target_following_count: i32,
    pub actor_following_count: i32,
    pub mutual_followers_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchRelationshipsRequest {
    pub target_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRelationshipItemDto {
    pub target_user_id: Uuid,
    pub state: RelationshipState,
    pub is_mutual: bool,
    pub is_followed_by_target: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRelationshipsResponseDto {
    pub items: Vec<BatchRelationshipItemDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SocialGraphListQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialGraphUserItemDto {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub barista_level: i32,
    pub barista_rank_title: String,
    pub account_tier: String,
    pub is_private_account: bool,
    pub relationship_state: RelationshipState,
    pub followed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowRequestItemDto {
    pub requester_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub barista_level: i32,
    pub barista_rank_title: String,
    pub account_tier: String,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialGraphCursorPageDto<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePrivacyRequest {
    pub is_private_account: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettingResponseDto {
    pub is_private_account: bool,
    pub pending_follow_requests_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRequestActionResponseDto {
    pub requester_id: Uuid,
    pub action: String,
    pub state: RelationshipState,
    pub pending_follow_requests_count: i32,
    pub followers_count: i32,
}
