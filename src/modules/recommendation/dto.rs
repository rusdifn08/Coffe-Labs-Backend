use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::{
    communities::dto::CommunitySummaryDto,
    social_feed::dto::SocialThreadItemDto,
};

#[derive(Debug, Clone, Deserialize)]
pub struct ForYouFeedQuery {
    pub cursor_offset: Option<usize>,
    pub limit: Option<i64>,
    pub method_filter: Option<String>,
    #[allow(dead_code)]
    pub refresh: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrendingFeedQuery {
    pub limit: Option<i64>,
    #[allow(dead_code)]
    pub window_hours: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkSeenRequest {
    #[serde(default)]
    pub thread_ids: Vec<Uuid>,
    #[serde(default)]
    pub engaged_thread_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedThreadItemDto {
    #[serde(flatten)]
    pub thread: SocialThreadItemDto,
    pub relevance_score: f64,
    pub taste_match_pct: u8,
    pub reason_badge: String,
    pub is_trending: bool,
    pub community_id: Option<Uuid>,
    pub community_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForYouFeedResponseDto {
    pub items: Vec<RecommendedThreadItemDto>,
    pub user_top_tastes: Vec<String>,
    pub next_offset: Option<usize>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingFeedResponseDto {
    pub items: Vec<RecommendedThreadItemDto>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedClubsResponseDto {
    pub items: Vec<CommunitySummaryDto>,
    pub user_top_tastes: Vec<String>,
}
