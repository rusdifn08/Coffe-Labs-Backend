use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GameBrewChapterDto {
    pub chapter_number: i16,
    pub slug: String,
    pub title: String,
    pub story_location: String,
    pub npc_mentor_name: String,
    pub target_recipe_slug: String,
    pub physics_config: serde_json::Value,
    pub story_dialogue: serde_json::Value,
    pub xp_reward_3_stars: i32,
    pub stars_earned: i16,
    pub high_score_accuracy_pct: f32,
    pub best_extraction_yield_pct: Option<f32>,
    pub is_unlocked: bool,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct SubmitGameBrewScoreRequest {
    #[validate(range(min = 0.0, max = 100.0))]
    pub pour_accuracy_pct: f32,
    #[validate(range(min = 10.0, max = 30.0))]
    pub extraction_yield_est: f32,
    #[serde(default)]
    pub thermal_stability_score: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
#[allow(dead_code)]
pub struct TelemetryPoint2Hz {
    pub t_sec: f32,
    pub flow_ml_s: f32,
    pub temp_c: f32,
    pub pitch_deg: f32,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct GameBrew3DSessionSubmissionDto {
    #[validate(range(min = 10.0, max = 900.0))]
    pub duration_sec: f32,
    #[validate(range(min = 20.0, max = 1200.0))]
    pub poured_total_ml: f32,
    #[validate(range(min = 0.0, max = 1.0))]
    pub laminar_flow_ratio: f32,
    #[validate(range(min = 0.0, max = 100.0))]
    pub flat_bed_score_pct: f32,
    #[validate(range(min = 0.0, max = 100.0))]
    pub spiral_evenness_pct: f32,
    #[serde(default)]
    pub swirl_performed: bool,
    #[serde(default)]
    pub bypass_events_count: i32,
    #[validate(range(min = 10.0, max = 30.0))]
    pub extraction_yield_est: f32,
    #[validate(range(min = 0.0, max = 100.0))]
    pub thermal_stability_score: f32,
    #[serde(default)]
    pub sample_curve_2hz: Vec<TelemetryPoint2Hz>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubmitGameBrewScoreResponse {
    pub chapter_number: i16,
    pub stars_earned: i16,
    pub final_accuracy_pct: f32,
    pub rank_title: String,
    pub unlocked_next_chapter: Option<i16>,
    pub xp_awarded: i32,
    pub mentor_feedback: String,
    pub physics_verified: bool,
}

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct BrewDoctorDiagnoseRequest {
    #[validate(length(min = 1, max = 8))]
    pub symptom_tags: Vec<String>,
    #[validate(range(min = 5.0, max = 60.0))]
    pub dose_g: f64,
    #[validate(range(min = 60.0, max = 1200.0))]
    pub water_ml: f64,
    #[validate(range(min = 65.0, max = 100.0))]
    pub temp_c: f64,
    #[validate(range(min = 30, max = 600))]
    pub target_sec: i32,
    #[validate(range(min = 30, max = 900))]
    pub actual_sec: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BrewDoctorDiagnosisResponse {
    pub diagnosis_code: String,
    pub diagnosis_title: String,
    pub x_extraction: f64,
    pub y_strength: f64,
    pub estimated_ey_pct: f64,
    pub estimated_tds_pct: f64,
    pub grind_clicks_delta: i32,
    pub temp_c_delta: i32,
    pub recommended_ratio: String,
    pub explanation: String,
    pub pour_technique_advice: String,
}
