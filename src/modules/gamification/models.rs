use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum XpActionType {
    BrewCompleted,
    MasterclassFinished,
    RecipeShared,
    StreakBonus,
    QuestReward,
}

impl XpActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BrewCompleted => "BREW_COMPLETED",
            Self::MasterclassFinished => "MASTERCLASS_FINISHED",
            Self::RecipeShared => "RECIPE_SHARED",
            Self::StreakBonus => "STREAK_BONUS",
            Self::QuestReward => "QUEST_REWARD",
        }
    }

    pub fn base_xp(&self) -> i32 {
        match self {
            Self::BrewCompleted => 120,
            Self::MasterclassFinished => 250,
            Self::RecipeShared => 150,
            Self::StreakBonus => 80,
            Self::QuestReward => 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GamificationProfileRow {
    pub user_id: Uuid,
    pub total_xp: i64,
    pub current_level: i32,
    pub current_streak: i32,
    pub longest_streak: i32,
    pub streak_freezes_available: i32,
    pub last_brew_local_date: Option<NaiveDate>,
    pub timezone: String,
    pub unlocked_badges_bitmap: Vec<u8>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct AchievementCatalogRow {
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
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BrewVerificationSessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recipe_id: Uuid,
    pub target_duration_sec: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_valid: bool,
    pub status: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct LeaderboardRow {
    pub rank: i64,
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub avatar_url: String,
    pub tier: String,
    pub total_xp: i64,
    pub current_level: i32,
    pub level_title: String,
    pub current_streak: i32,
    pub badges_count: i32,
}
