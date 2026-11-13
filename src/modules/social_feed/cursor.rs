use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::social_feed::error::SocialFeedError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineCursor {
    pub last_id: Uuid,
    pub pinned_passed: bool,
}

impl TimelineCursor {
    pub fn new(last_id: Uuid, pinned_passed: bool) -> Self {
        Self {
            last_id,
            pinned_passed,
        }
    }

    pub fn encode_base64_url(&self) -> Result<String, SocialFeedError> {
        let json = serde_json::to_vec(self)
            .map_err(|_| SocialFeedError::BadRequest("Gagal menyandikan cursor".into()))?;
        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode_base64_url(raw: &str) -> Result<Self, SocialFeedError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.len() > 256 {
            return Err(SocialFeedError::InvalidCursor);
        }
        let decoded = URL_SAFE_NO_PAD
            .decode(trimmed)
            .map_err(|_| SocialFeedError::InvalidCursor)?;
        serde_json::from_slice::<Self>(&decoded).map_err(|_| SocialFeedError::InvalidCursor)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryCursor {
    pub total_xp: i64,
    pub user_id: Uuid,
}

impl DirectoryCursor {
    pub fn encode_base64_url(&self) -> Result<String, SocialFeedError> {
        let json = serde_json::to_vec(self)
            .map_err(|_| SocialFeedError::BadRequest("Gagal menyandikan cursor direktori".into()))?;
        Ok(URL_SAFE_NO_PAD.encode(json))
    }

    pub fn decode_base64_url(raw: &str) -> Result<Self, SocialFeedError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.len() > 256 {
            return Err(SocialFeedError::InvalidCursor);
        }
        let decoded = URL_SAFE_NO_PAD
            .decode(trimmed)
            .map_err(|_| SocialFeedError::InvalidCursor)?;
        serde_json::from_slice::<Self>(&decoded).map_err(|_| SocialFeedError::InvalidCursor)
    }
}
