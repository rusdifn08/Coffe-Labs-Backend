use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::modules::social_graph::error::SocialGraphError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FollowEdgeStatus {
    Accepted,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipState {
    SelfProfile,
    None,
    FollowedByTarget,
    Requested,
    Following,
    Mutual,
}

impl RelationshipState {
    pub fn resolve(
        viewer_id: Option<Uuid>,
        target_id: Uuid,
        out_accepted: bool,
        in_accepted: bool,
        out_pending: bool,
    ) -> Self {
        if let Some(vid) = viewer_id {
            if vid == target_id {
                return Self::SelfProfile;
            }
        } else {
            return Self::None;
        }
        if out_accepted && in_accepted {
            Self::Mutual
        } else if out_accepted {
            Self::Following
        } else if out_pending {
            Self::Requested
        } else if in_accepted {
            Self::FollowedByTarget
        } else {
            Self::None
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SelfProfile => "self_profile",
            Self::None => "none",
            Self::FollowedByTarget => "followed_by_target",
            Self::Requested => "requested",
            Self::Following => "following",
            Self::Mutual => "mutual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphCursor {
    pub created_at: DateTime<Utc>,
    pub user_id: Uuid,
}

impl GraphCursor {
    pub fn encode(&self) -> String {
        let raw = format!("{}|{}", self.created_at.timestamp_micros(), self.user_id);
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    pub fn decode(encoded: &str) -> Result<Self, SocialGraphError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded.trim().as_bytes())
            .map_err(|_| SocialGraphError::BadRequest("Cursor paginasi tidak valid.".to_string()))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| SocialGraphError::BadRequest("Encoding cursor tidak valid.".to_string()))?;
        let mut parts = text.splitn(2, '|');
        let micros_str = parts
            .next()
            .ok_or_else(|| SocialGraphError::BadRequest("Format cursor salah.".to_string()))?;
        let uuid_str = parts
            .next()
            .ok_or_else(|| SocialGraphError::BadRequest("UUID cursor tidak ditemukan.".to_string()))?;
        let micros: i64 = micros_str
            .parse()
            .map_err(|_| SocialGraphError::BadRequest("Timestamp cursor tidak valid.".to_string()))?;
        let created_at = DateTime::from_timestamp_micros(micros)
            .ok_or_else(|| SocialGraphError::BadRequest("Timestamp cursor di luar rentang.".to_string()))?;
        let user_id = Uuid::parse_str(uuid_str)
            .map_err(|_| SocialGraphError::BadRequest("UUID cursor tidak valid.".to_string()))?;
        Ok(Self { created_at, user_id })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EdgePairState {
    pub out_accepted: bool,
    pub in_accepted: bool,
    pub out_pending: bool,
    pub in_pending: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct TargetUserRow {
    pub user_id: Uuid,
    pub is_private_account: bool,
    pub followers_count: i32,
    pub following_count: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct ActorSummaryRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub following_count: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct MiniUserProfileRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub barista_level: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct FollowEdgeUserRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub barista_level: i32,
    pub account_tier: String,
    pub is_private_account: bool,
    pub edge_created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FollowMutationOutcome {
    pub changed: bool,
    pub resulting_status: Option<FollowEdgeStatus>,
    pub target_followers_count: i32,
    pub target_following_count: i32,
    pub actor_following_count: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct PrivacySettingRow {
    pub is_private_account: bool,
    pub pending_follow_requests_count: i32,
}
