use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SocialPostResponse {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub author_role: String,
    pub content: String,
    pub recipe_id: Option<i32>,
    pub recipe_snapshot: Option<serde_json::Value>,
    pub likes_count: i32,
    pub comments_count: i32,
    pub copies_count: i32,
    pub reposts_count: i32,
    pub tags: Vec<String>,
    pub is_liked_by_me: bool,
    pub is_reposted_by_me: bool,
    pub is_saved_by_me: bool,
    pub is_edited: bool,
    pub next_cursor: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateSocialPostRequest {
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
    pub author_role: Option<String>,
    #[validate(length(min = 1, max = 4000, message = "Konten postingan tidak boleh kosong."))]
    pub content: String,
    pub recipe_id: Option<i32>,
    pub recipe_snapshot: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateSocialPostRequest {
    pub author_name: Option<String>,
    #[validate(length(min = 1, max = 4000, message = "Konten postingan tidak boleh kosong."))]
    pub content: String,
    pub recipe_id: Option<i32>,
    pub recipe_snapshot: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default, Clone, ToSchema)]
pub struct DeleteSocialPostQuery {
    pub username: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ToggleUserPostActionRequest {
    #[validate(length(min = 1, max = 100, message = "Username diperlukan."))]
    pub username: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RepostPostResponse {
    pub is_reposted: bool,
    pub reposts_count: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SavePostResponse {
    pub is_saved: bool,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ReportPostRequest {
    #[validate(length(min = 1, max = 100, message = "Username pelapor diperlukan."))]
    pub username: String,
    #[validate(length(min = 3, max = 255, message = "Alasan laporan minimal 3 karakter."))]
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReportPostResponse {
    pub reported: bool,
    pub post_id: Uuid,
}

#[derive(Debug, Deserialize, Default, Clone, ToSchema)]
pub struct SocialFeedQuery {
    pub sort: Option<String>,
    pub tag: Option<String>,
    pub username: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LikePostRequest {
    #[validate(length(min = 1, max = 100, message = "Username diperlukan untuk memberikan like."))]
    pub username: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LikePostResponse {
    pub is_liked: bool,
    pub likes_count: i32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateCommentRequest {
    pub author_name: Option<String>,
    pub author_avatar: Option<String>,
    pub parent_id: Option<Uuid>,
    #[validate(length(min = 1, max = 2000, message = "Komentar tidak boleh kosong."))]
    pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CopyRecipeResponse {
    pub copies_count: i32,
    pub recipe_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct FollowToggleRequest {
    #[validate(length(min = 1, max = 100))]
    pub follower_username: String,
    #[validate(length(min = 1, max = 100))]
    pub following_username: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FollowStatsResponse {
    pub username: String,
    pub followers_count: i64,
    pub following_count: i64,
    pub is_following: bool,
}
