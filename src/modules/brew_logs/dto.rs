use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::modules::brew_logs::models::MethodFrequency;

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct BrewLogQuery {
    pub username: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BrewAnalyticsResponse {
    pub total_brews: i64,
    pub total_journal_entries: i64,
    pub average_sensory_rating: f64,
    pub method_breakdown: Vec<MethodFrequency>,
}
