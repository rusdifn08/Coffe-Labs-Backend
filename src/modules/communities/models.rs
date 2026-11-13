use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

pub fn normalize_privacy_level(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "PUBLIC" => Some("PUBLIC"),
        "RESTRICTED" => Some("RESTRICTED"),
        "SECRET_VAULT" | "SECRET" => Some("SECRET_VAULT"),
        _ => None,
    }
}

pub fn normalize_club_role(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "OWNER" => Some("OWNER"),
        "ADMIN" => Some("ADMIN"),
        "MODERATOR" => Some("MODERATOR"),
        "BARISTA_MEMBER" | "MEMBER" => Some("BARISTA_MEMBER"),
        _ => None,
    }
}

pub fn is_privileged_role(role: &str) -> bool {
    matches!(role, "OWNER" | "ADMIN" | "MODERATOR")
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityRow {
    pub id: Uuid,
    pub handle_slug: String,
    pub name: String,
    pub description: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub privacy_level: String,
    pub owner_id: Uuid,
    pub focus_tags: serde_json::Value,
    pub taste_feature_vector: Vec<f32>,
    pub members_count: i32,
    pub threads_count: i32,
    pub shared_recipes_count: i32,
    pub total_club_xp: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityDiscoverRow {
    pub id: Uuid,
    pub handle_slug: String,
    pub name: String,
    pub description: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub privacy_level: String,
    pub owner_id: Uuid,
    pub focus_tags: serde_json::Value,
    pub taste_feature_vector: Vec<f32>,
    pub members_count: i32,
    pub threads_count: i32,
    pub shared_recipes_count: i32,
    pub total_club_xp: i64,
    pub viewer_role: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityChannelRow {
    pub id: Uuid,
    pub community_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub linked_conversation_id: Option<Uuid>,
    pub display_order: i16,
    pub is_readonly_announcement: bool,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityInviteSecretRow {
    pub id: Uuid,
    pub community_id: Uuid,
    #[allow(dead_code)]
    pub created_by: Uuid,
    pub secret_code_display: String,
    pub invite_url_token: String,
    pub role_granted: String,
    pub max_uses: Option<i32>,
    pub used_count: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_revoked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CommunityLeaderboardRow {
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
}

#[derive(Debug, Clone, FromRow)]
pub struct ClaimedInviteRow {
    #[allow(dead_code)]
    pub invite_id: Uuid,
    pub community_id: Uuid,
    pub assigned_role: String,
    pub already_member: bool,
}
