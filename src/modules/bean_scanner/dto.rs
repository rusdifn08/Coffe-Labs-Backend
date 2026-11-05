use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlockRole {
    Roastery,
    OriginOrTitle,
    Process,
    TastingNote,
    Variety,
    Altitude,
    IgnoredNoise,
}

impl BlockRole {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Roastery => "🏪 Roastery",
            Self::OriginOrTitle => "⛰️ Origin / Kopi",
            Self::Process => "🍷 Proses",
            Self::TastingNote => "✨ Tasting Note",
            Self::Variety => "🌱 Varietas",
            Self::Altitude => "📐 Altitude",
            Self::IgnoredNoise => "Noise (Diabaikan)",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Roastery => "#D4AF37",
            Self::OriginOrTitle => "#4E8752",
            Self::Process => "#9E2A3B",
            Self::TastingNote => "#3B82A6",
            Self::Variety => "#2E7D32",
            Self::Altitude => "#5C6BC0",
            Self::IgnoredNoise => "#757575",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OcrTextBlockDto {
    pub text: String,
    pub normalized_height: f32,
    pub top_y: f32,
    #[serde(default)]
    pub left_x: f32,
    #[serde(default)]
    pub normalized_width: f32,
    #[serde(default = "default_confidence")]
    pub confidence: f32,
}

fn default_confidence() -> f32 {
    0.95
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClassifiedOcrBlockDto {
    pub text: String,
    pub role: BlockRole,
    pub role_label: String,
    pub color_hex: String,
    pub top_y: f32,
    pub left_x: f32,
    pub normalized_width: f32,
    pub normalized_height: f32,
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
    #[serde(default = "default_notes_source")]
    pub notes_source: String,
    #[serde(default)]
    pub is_ai_inferred_altitude: bool,
    #[serde(default)]
    pub is_ai_inferred_variety: bool,
    #[serde(default)]
    pub classified_blocks: Vec<ClassifiedOcrBlockDto>,
}

fn default_notes_source() -> String {
    "AI_SOMMELIER_INFERRED".to_string()
}
