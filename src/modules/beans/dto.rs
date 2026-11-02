use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::beans::models::DialInPourStep;

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct BeanCatalogQuery {
    pub q: Option<String>,
    pub roastery: Option<String>,
    pub city: Option<String>,
    pub process: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
#[allow(dead_code)]
pub struct DialInCalculationRequest {
    #[validate(length(min = 1, max = 180))]
    pub bean_name: String,
    pub origin: Option<String>,
    pub process: Option<String>,
    pub roast_level: Option<String>,
    pub dripper: Option<String>,
    pub flavor_goal: Option<String>,
    pub coffee_grams: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DialInRecommendationResponse {
    pub bean_name: String,
    pub dripper: String,
    pub flavor_goal: String,
    pub coffee_grams: f64,
    pub total_water_grams: f64,
    pub ratio: String,
    pub water_temp_celsius: i32,
    pub grind_size_label: String,
    pub grind_microns: i32,
    pub comandante_clicks: i32,
    pub target_tds_percent: f64,
    pub target_extraction_yield_percent: f64,
    pub total_time_seconds: i32,
    pub barista_note: String,
    pub pour_schedule: Vec<DialInPourStep>,
}
