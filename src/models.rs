use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub role: String,   // "admin" | "user"
    pub status: String, // "pending" | "approved" | "declined"
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSummary {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub status: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserSummary {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
            role: u.role,
            status: u.status,
            avatar_url: u.avatar_url,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserSummary,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserStatusRequest {
    pub status: String, // "approved" | "declined"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user id
    pub username: String,
    pub role: String,
    pub status: String,
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Feedback {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub username: String,
    pub user_role: String,
    pub category: String, // "bug_report" | "feature_request" | "recipe_feedback" | "ui_ux" | "other"
    pub feature_affected: String,
    pub title: String,
    pub description: String,
    pub rating: Option<i32>,
    pub app_version: Option<String>,
    pub device_info: Option<String>,
    pub status: String, // "pending" | "reviewed" | "in_progress" | "resolved" | "dismissed"
    pub admin_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFeedbackRequest {
    pub category: String,
    pub feature_affected: String,
    pub title: String,
    pub description: String,
    pub rating: Option<i32>,
    pub app_version: Option<String>,
    pub device_info: Option<String>,
    pub guest_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFeedbackStatusRequest {
    pub status: String,
    pub admin_notes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FeedbackQuery {
    pub status: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedbackStats {
    pub total: i64,
    pub pending: i64,
    pub in_progress: i64,
    pub resolved: i64,
    pub bug_reports: i64,
    pub feature_requests: i64,
    pub average_rating: f64,
}

// ================= VERSION GATEKEEPER MODELS =================

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct AppVersionConfig {
    pub id: String,
    pub platform: String,
    pub min_base_version: String,
    pub latest_base_version: String,
    pub min_build_number: i32,
    pub latest_build_number: i32,
    pub latest_patch_number: i32,
    pub is_force_update: bool,
    pub is_patch_required: bool,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Default)]
pub struct VersionCheckQuery {
    pub platform: Option<String>,       // "android" | "ios" | "windows"
    pub base_version: Option<String>,   // contoh: "1.0.0"
    pub build_number: Option<i32>,      // contoh: 1
    pub patch_number: Option<i32>,      // nomor patch Shorebird saat ini
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpdateAction {
    None,
    SilentOtaPatch,     // Unduh patch Shorebird di background
    ForceOtaPatch,      // Tahan layar, unduh patch Shorebird + progress UI, lalu restart
    FlexibleStoreUpdate,// Trigger Native In-App Update / unduhan background
    ForceStoreUpdate,   // Blokir layar, wajibkan Native Update / APK Stream
}

#[derive(Debug, Serialize)]
pub struct VersionCheckResponse {
    pub action: UpdateAction,
    pub action_code: String,
    pub target_patch: Option<i32>,
    pub latest_base_version: String,
    pub latest_build_number: i32,
    pub is_force_update: bool,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAppVersionConfigRequest {
    pub min_base_version: Option<String>,
    pub latest_base_version: Option<String>,
    pub min_build_number: Option<i32>,
    pub latest_build_number: Option<i32>,
    pub latest_patch_number: Option<i32>,
    pub is_force_update: Option<bool>,
    pub is_patch_required: Option<bool>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
}

// ================= SOCIAL COFFEE MODELS =================

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SocialPost {
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
    pub tags: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub tags: Vec<String>,
    pub is_liked_by_me: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSocialPostRequest {
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub author_role: Option<String>,
    pub content: String,
    pub recipe_id: Option<i32>,
    pub recipe_snapshot: Option<serde_json::Value>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SocialFeedQuery {
    pub sort: Option<String>, // "latest" | "popular" | "recipes"
    pub tag: Option<String>,
    pub username: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LikePostRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct LikePostResponse {
    pub is_liked: bool,
    pub likes_count: i32,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SocialComment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Option<Uuid>,
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub author_name: String,
    pub author_avatar: Option<String>,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CopyRecipeResponse {
    pub copies_count: i32,
    pub recipe_snapshot: Option<serde_json::Value>,
}
