use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OcrTextBlockDto {
    pub text: String,
    pub normalized_height: f32,
    pub top_y: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BeanScanRequestDto {
    #[validate(length(min = 1, message = "Teks OCR tidak boleh kosong"))]
    pub raw_full_text: String,
    #[serde(default)]
    pub blocks: Vec<OcrTextBlockDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecommendedRecipeSummary {
    pub slug: String,
    pub name: String,
    pub target_notes: String,
    pub suggested_temp_c: i32,
    pub ratio: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ResolvedCoffeeBeanResponse {
    pub bean_id: Option<Uuid>,
    pub is_catalog_verified: bool,
    pub confidence_score: f32,
    pub roastery_name: String,
    pub bean_name: String,
    pub origin: String,
    pub variety: String,
    pub process: String,
    pub roast_level: String,
    pub altitude_masl: Option<String>,
    pub tasting_notes: Vec<String>,
    pub recommended_water_temp_c: i32,
    pub recommended_recipes: Vec<RecommendedRecipeSummary>,
}
