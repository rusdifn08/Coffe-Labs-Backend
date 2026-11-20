use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct StartVerifiedBrewDto {
    #[validate(length(min = 1, max = 50))]
    pub method_id: String,
    #[validate(length(min = 1, max = 80))]
    pub method_display_name: String,
    pub archetype: Option<String>,
    #[validate(length(min = 1, max = 80))]
    pub recipe_id: String,
    #[validate(length(min = 1, max = 160))]
    pub recipe_title: String,
    #[validate(range(min = 15, max = 86400))]
    pub target_duration_sec: i32,
    pub total_steps_count: Option<i16>,
    pub bean_name: Option<String>,
    pub roastery_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartVerifiedBrewResponseDto {
    pub session_id: Uuid,
    pub server_started_at: DateTime<Utc>,
    pub min_valid_duration_sec: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompleteVerifiedBrewDto {
    pub active_elapsed_ms: i64,
    pub paused_ms: Option<i64>,
    pub completed_steps_mask: Option<i64>,
    pub accuracy_score: Option<f32>,
    pub coffee_dose_g: Option<f32>,
    pub water_total_ml: Option<f32>,
    pub water_temp_c: Option<i16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct XpBreakdownItemDto {
    pub label: String,
    pub xp: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GamificationRewardDeltaDto {
    pub source_kind: String,
    pub source_label: String,
    pub xp_before: i64,
    pub xp_gained: i32,
    pub xp_after: i64,
    pub level_before: i32,
    pub level_after: i32,
    pub did_level_up: bool,
    pub rank_before: String,
    pub rank_after: String,
    pub progress_ratio_before: f64,
    pub progress_ratio_after: f64,
    pub method_id: Option<String>,
    pub method_new_completed_count: Option<i32>,
    pub new_streak_days: i32,
    pub breakdown_items: Vec<XpBreakdownItemDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TriedRecipeDto {
    pub recipe_id: String,
    pub recipe_title: String,
    pub method_id: String,
    pub times_completed: i32,
    pub best_accuracy_pct: Option<f32>,
    pub last_duration_sec: i32,
    pub last_completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GamificationSummaryDto {
    pub total_xp: i64,
    pub current_level: i32,
    pub barista_rank_title: String,
    pub level_start_xp: i64,
    pub next_level_xp: i64,
    pub level_progress_ratio: f64,
    pub total_verified_brews_count: i32,
    pub current_streak_days: i32,
    pub method_completed_counts: HashMap<String, i32>,
    pub recently_tried_recipes: Vec<TriedRecipeDto>,
}
