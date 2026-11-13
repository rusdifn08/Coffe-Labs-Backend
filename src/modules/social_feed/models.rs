use chrono::{DateTime, Utc};
use roaring::RoaringBitmap;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::io::Cursor;
use uuid::Uuid;

pub fn xp_req_for_level(level: u32) -> i64 {
    if level <= 1 {
        return 0;
    }
    (100.0 * (level as f64).powf(1.65)).floor() as i64
}

pub fn level_from_total_xp(total_xp: i64) -> i32 {
    if total_xp <= 0 {
        return 1;
    }
    let mut low = 1u32;
    let mut high = 120u32;
    let mut best = 1u32;
    while low <= high {
        let mid = (low + high) / 2;
        if xp_req_for_level(mid) <= total_xp {
            best = mid;
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    best as i32
}

pub fn barista_rank_for_level(level: i32) -> &'static str {
    match level {
        i32::MIN..=9 => "Home Brewer",
        10..=24 => "Pour-Over Artisan",
        25..=49 => "Sensory Alchemist",
        50..=79 => "Head Roaster & Master",
        80..=i32::MAX => "WBrC Grandmaster",
    }
}

pub fn compute_level_progress_ratio(level: i32, total_xp: i64) -> f32 {
    let lvl = level.max(1) as u32;
    let current_req = xp_req_for_level(lvl);
    let next_req = xp_req_for_level(lvl + 1);
    let span = (next_req - current_req).max(1) as f64;
    let num = (total_xp - current_req).max(0) as f64;
    (num / span).clamp(0.0, 1.0) as f32
}

pub fn decode_roaring_ids(bytes: Option<&[u8]>) -> Vec<u32> {
    let Some(raw) = bytes else {
        return Vec::new();
    };
    if raw.is_empty() {
        return Vec::new();
    }
    match RoaringBitmap::deserialize_from(Cursor::new(raw)) {
        Ok(bitmap) => bitmap.iter().collect(),
        Err(_) => Vec::new(),
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct UserProfileProjectionRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub account_tier: String,
    pub total_xp: i64,
    pub barista_level: i32,
    pub current_streak_days: i32,
    pub unlocked_badges_bitmap: Option<Vec<u8>>,
    pub top_featured_badges: serde_json::Value,
    pub followers_count: i32,
    pub following_count: i32,
    pub total_threads_count: i32,
    pub total_brews_count: i32,
    pub total_recipes_created_count: i32,
    pub total_recipe_tries_received: i32,
    pub recent_brew_snapshot: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ProfileDirectoryRow {
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
    pub recent_brew_snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SocialThreadWithAuthorRow {
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
    pub attachment_payload: Option<serde_json::Value>,
    pub likes_count: i32,
    pub replies_count: i32,
    pub reposts_count: i32,
    pub try_recipe_count: i32,
    pub is_pinned_on_profile: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BadgeCatalogMiniRow {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub rarity: String,
    pub rive_artboard: Option<String>,
}
