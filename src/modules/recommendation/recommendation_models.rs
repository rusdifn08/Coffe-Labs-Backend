use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::modules::social_feed::models::SocialThreadWithAuthorRow;

#[derive(Debug, Clone, FromRow)]
pub struct ScoredThreadCandidateRow {
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
    pub depth_level: i16,
    pub body_text: String,
    pub has_media: bool,
    pub media_urls: serde_json::Value,
    pub attachment_type: Option<String>,
    pub attachment_payload: Option<serde_json::Value>,
    pub likes_count: i32,
    pub replies_count: i32,
    pub reposts_count: i32,
    pub try_recipe_count: i32,
    pub is_pinned_on_profile: bool,
    pub created_at: DateTime<Utc>,
    pub community_id: Option<Uuid>,
    pub community_name: Option<String>,
    pub trending_gravity_score: f64,
    pub taste_feature_vector: Vec<f32>,
}

impl ScoredThreadCandidateRow {
    pub fn to_author_row(&self) -> SocialThreadWithAuthorRow {
        SocialThreadWithAuthorRow {
            id: self.id,
            author_id: self.author_id,
            author_username: self.author_username.clone(),
            author_display_name: self.author_display_name.clone(),
            author_avatar_url: self.author_avatar_url.clone(),
            author_tier: self.author_tier.clone(),
            author_barista_level: self.author_barista_level,
            author_barista_rank_title: self.author_barista_rank_title.clone(),
            kind: self.kind.clone(),
            parent_thread_id: self.parent_thread_id,
            root_thread_id: self.root_thread_id,
            quoted_thread_id: self.quoted_thread_id,
            depth_level: self.depth_level,
            body_text: self.body_text.clone(),
            has_media: self.has_media,
            media_urls: self.media_urls.clone(),
            attachment_payload: self.attachment_payload.clone(),
            likes_count: self.likes_count,
            replies_count: self.replies_count,
            reposts_count: self.reposts_count,
            try_recipe_count: self.try_recipe_count,
            is_pinned_on_profile: self.is_pinned_on_profile,
            created_at: self.created_at,
        }
    }
}
