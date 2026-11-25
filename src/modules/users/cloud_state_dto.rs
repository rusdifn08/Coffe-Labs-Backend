use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserCloudStateResponse {
    pub user_id: Uuid,
    pub username: String,
    pub full_name: String,
    pub avatar_url: String,
    pub total_exp: i32,
    pub current_level: i32,
    pub level_title: String,
    pub brews_count: i32,
    pub toolkit_count: i32,
    pub cupping_count: i32,
    pub streak_days: i32,
    pub last_activity_date: Option<String>,
    pub brew_history: Value,
    pub brew_journal: Value,
    pub favorite_recipe_ids: Vec<i64>,
    pub custom_recipes: Value,
    pub deleted_default_ids: Vec<i64>,
    pub game_brew_progress: Value,
    pub state_version: i64,
    pub updated_at: DateTime<Utc>,
    pub has_cloud_record: bool,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpsertUserCloudStateRequest {
    #[validate(range(min = 0, max = 10_000_000, message = "total_exp tidak valid."))]
    pub total_exp: i32,
    #[validate(range(min = 0, max = 1_000_000, message = "brews_count tidak valid."))]
    pub brews_count: i32,
    #[validate(range(min = 0, max = 1_000_000, message = "toolkit_count tidak valid."))]
    pub toolkit_count: i32,
    #[validate(range(min = 0, max = 1_000_000, message = "cupping_count tidak valid."))]
    pub cupping_count: i32,
    #[validate(range(min = 1, max = 36_500, message = "streak_days minimal 1."))]
    pub streak_days: i32,
    #[validate(length(max = 16, message = "Format tanggal maksimal 16 karakter."))]
    pub last_activity_date: Option<String>,
    pub brew_history: Option<Value>,
    pub brew_journal: Option<Value>,
    pub favorite_recipe_ids: Option<Vec<i64>>,
    pub custom_recipes: Option<Value>,
    pub deleted_default_ids: Option<Vec<i64>>,
    pub game_brew_progress: Option<Value>,
    pub base_state_version: Option<i64>,
    #[validate(length(min = 1, max = 100, message = "Nama lengkap harus 1-100 karakter."))]
    pub full_name: Option<String>,
    #[validate(length(max = 1024, message = "URL avatar maksimal 1024 karakter."))]
    pub avatar_url: Option<String>,
}

/// Kurva 10 level resmi Coffee Labs (identik 1:1 dengan `CoffeeLevel.allLevels` di Flutter).
pub fn compute_coffee_level(total_exp: i32) -> (i32, &'static str) {
    match total_exp.max(0) {
        0..=199 => (1, "Novice Brewer"),
        200..=499 => (2, "Aroma Explorer"),
        500..=999 => (3, "Extraction Craftsman"),
        1000..=1799 => (4, "Dial-In Alchemist"),
        1800..=2999 => (5, "Sensory Sommelier"),
        3000..=4499 => (6, "Master of TDS & Solutes"),
        4500..=6499 => (7, "Terroir & Origin Virtuoso"),
        6500..=8999 => (8, "Championship Brewer"),
        9000..=12499 => (9, "Grandmaster of Specialty Coffee"),
        _ => (10, "Legendary Q-Grader & Coffee Luminary"),
    }
}

pub fn sanitize_json_array(raw: Option<&Value>, max_items: usize) -> Value {
    match raw {
        Some(Value::Array(items)) => {
            let sliced = items.iter().take(max_items).cloned().collect::<Vec<_>>();
            Value::Array(sliced)
        }
        _ => Value::Array(Vec::new()),
    }
}

pub fn sanitize_id_list(raw: Option<&[i64]>, max_items: usize) -> Vec<i64> {
    let Some(list) = raw else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(list.len().min(max_items));
    for &id in list {
        if id > 0 && seen.insert(id) {
            out.push(id);
            if out.len() >= max_items {
                break;
            }
        }
    }
    out
}

pub fn extract_id_list_from_json(val: &Value) -> Vec<i64> {
    val.as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).filter(|&v| v > 0).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn level_curve_matches_mobile_ten_tiers() {
        assert_eq!(compute_coffee_level(0), (1, "Novice Brewer"));
        assert_eq!(compute_coffee_level(200), (2, "Aroma Explorer"));
        assert_eq!(compute_coffee_level(500), (3, "Extraction Craftsman"));
        assert_eq!(compute_coffee_level(1720), (4, "Dial-In Alchemist"));
        assert_eq!(compute_coffee_level(1800), (5, "Sensory Sommelier"));
        assert_eq!(compute_coffee_level(12500), (10, "Legendary Q-Grader & Coffee Luminary"));
    }

    #[test]
    fn sanitize_helpers_bound_and_deduplicate() {
        let arr = json!([1, 2, 3, 4, 5]);
        assert_eq!(sanitize_json_array(Some(&arr), 3), json!([1, 2, 3]));
        assert_eq!(sanitize_json_array(Some(&json!({"bad": true})), 3), json!([]));
        assert_eq!(sanitize_id_list(Some(&[-1, 0, 10, 10, 22, 5]), 2), vec![10, 22]);
    }
}
