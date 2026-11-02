use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct StartBrewSessionRequest {
    pub recipe_id: Option<Uuid>,
    #[validate(range(min = 15, max = 3600, message = "Target durasi resep harus antara 15 hingga 3600 detik."))]
    pub target_duration_sec: i32,
    #[validate(length(min = 3, max = 64))]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StartBrewSessionResponse {
    pub session_id: Uuid,
    pub session_token: String,
    pub idempotency_key: String,
    pub recipe_id: Uuid,
    pub target_duration_sec: i32,
    pub min_valid_duration_sec: i32,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CompleteBrewSessionRequest {
    pub session_id: Uuid,
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    pub actual_duration_sec: Option<i32>,
    pub origin: Option<String>,
    pub variety: Option<String>,
    pub process: Option<String>,
    pub technique: Option<String>,
    pub water_temp_c: Option<i32>,
    pub sensory_score: Option<f64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ClaimDomainActionRequest {
    #[validate(length(min = 3, max = 32))]
    pub action_type: String,
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    pub reference_id: Option<Uuid>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BadgeDto {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub criteria_json: Value,
    pub xp_reward: i32,
    pub rive_artboard: String,
    pub rarity: String,
    pub tier_required: String,
    pub is_unlocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GamificationProfileResponse {
    pub user_id: Uuid,
    pub username: String,
    pub tier: String,
    pub tier_multiplier: f64,
    pub total_xp: i64,
    pub current_level: i32,
    pub level_title: String,
    pub current_level_floor_xp: i64,
    pub next_level_xp: i64,
    pub level_progress_pct: f64,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub streak_freezes_available: i32,
    pub last_brew_local_date: Option<NaiveDate>,
    pub timezone: String,
    pub unlocked_badge_ids: Vec<u32>,
    pub badges: Vec<BadgeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct XpRewardResponse {
    pub transaction_ulid: String,
    pub action_type: String,
    pub base_xp: i32,
    pub tier_multiplier: f64,
    pub streak_bonus_xp: i32,
    pub badges_bonus_xp: i32,
    pub xp_gained: i32,
    pub total_xp: i64,
    pub previous_level: i32,
    pub new_level: i32,
    pub leveled_up: bool,
    pub level_title: String,
    pub current_level_floor_xp: i64,
    pub next_level_xp: i64,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub streak_updated: bool,
    pub streak_status: String,
    pub streak_freezes_available: i32,
    pub newly_unlocked_badges: Vec<BadgeDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LeaderboardQuery {
    pub scope: Option<String>, // "global" | "weekly"
    pub limit: Option<i64>,
}
