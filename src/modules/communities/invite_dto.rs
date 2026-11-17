use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use super::dto::JoinClubResponseDto;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct InviteUsersToClubRequest {
    #[validate(length(min = 1, max = 50, message = "Pilih 1 sampai 50 barista untuk diundang."))]
    pub user_ids: Vec<Uuid>,
    #[serde(default)]
    pub role_granted: Option<String>,
    #[serde(default)]
    #[validate(length(max = 280, message = "Pesan undangan maksimal 280 karakter."))]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubMemberInvitationDto {
    pub id: Uuid,
    pub community_id: Uuid,
    pub community_name: String,
    pub community_slug: String,
    pub community_description: String,
    pub privacy_level: String,
    pub members_count: i32,
    pub focus_tags: Vec<String>,
    pub inviter_id: Uuid,
    pub inviter_username: String,
    pub inviter_display_name: String,
    pub inviter_avatar_url: Option<String>,
    pub role_granted: String,
    pub message: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClubSentInvitationDto {
    pub id: Uuid,
    pub community_id: Uuid,
    pub invitee_id: Uuid,
    pub invitee_username: String,
    pub invitee_display_name: String,
    pub invitee_avatar_url: Option<String>,
    pub role_granted: String,
    pub status: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendClubInvitesResponseDto {
    pub invited_count: usize,
    pub skipped_already_member: usize,
    pub invitations: Vec<ClubSentInvitationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RespondInvitationResponseDto {
    pub invitation_id: Uuid,
    pub community_id: Uuid,
    pub status: String,
    pub joined_club: Option<JoinClubResponseDto>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ClubMemberInvitationRow {
    pub id: Uuid,
    pub community_id: Uuid,
    pub community_name: String,
    pub community_slug: String,
    pub community_description: String,
    pub privacy_level: String,
    pub members_count: i32,
    pub focus_tags: serde_json::Value,
    pub inviter_id: Uuid,
    pub inviter_username: String,
    pub inviter_display_name: String,
    pub inviter_avatar_url: Option<String>,
    pub role_granted: String,
    pub message: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ClubSentInvitationRow {
    pub id: Uuid,
    pub community_id: Uuid,
    pub invitee_id: Uuid,
    pub invitee_username: String,
    pub invitee_display_name: String,
    pub invitee_avatar_url: Option<String>,
    pub role_granted: String,
    pub status: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
