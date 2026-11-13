use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeStepMiniDto {
    pub step_number: u16,
    pub title: String,
    pub duration_sec: u32,
    pub water_added_ml: f32,
    #[serde(default)]
    pub target_temp_c: Option<u16>,
    #[serde(default)]
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PourCurvePoint {
    pub t_sec: u16,
    pub actual_ml: f32,
    pub target_ml: f32,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum ThreadAttachmentPayload {
    #[serde(rename = "interactive_recipe_card")]
    InteractiveRecipeCard {
        recipe_id: String,
        title: String,
        brewer_name: String,
        method: String,
        coffee_dose_g: f32,
        total_water_ml: f32,
        primary_temp_c: u16,
        #[serde(default)]
        secondary_temp_c: Option<u16>,
        target_duration_sec: u32,
        #[serde(default)]
        steps_preview: Vec<RecipeStepMiniDto>,
    },
    #[serde(rename = "pouring_telemetry_curve")]
    PouringTelemetryCurve {
        session_id: Uuid,
        recipe_title: String,
        bean_name: String,
        accuracy_score_pct: f32,
        total_duration_sec: u32,
        #[serde(default)]
        curve_points: Vec<PourCurvePoint>,
    },
    #[serde(rename = "scanned_coffee_bag_tag")]
    ScannedCoffeeBagTag {
        class_slug: String,
        roastery: String,
        bean_name: String,
        origin: String,
        process: String,
        altitude_masl: String,
        #[serde(default)]
        tasting_notes: Vec<String>,
        confidence_pct: f32,
    },
}

impl ThreadAttachmentPayload {
    pub fn attachment_type_tag(&self) -> &'static str {
        match self {
            Self::InteractiveRecipeCard { .. } => "RECIPE_CARD",
            Self::PouringTelemetryCurve { .. } => "POURING_TELEMETRY",
            Self::ScannedCoffeeBagTag { .. } => "SCANNED_BEAN_TAG",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaItemDto {
    pub url: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub blurhash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentBrewSnapshot {
    #[serde(default)]
    pub recipe_id: Option<String>,
    pub recipe_title: String,
    pub bean_name: String,
    pub roastery: String,
    pub process: String,
    pub method: String,
    #[serde(default)]
    pub primary_temp_c: Option<u16>,
    #[serde(default)]
    pub secondary_temp_c: Option<u16>,
    #[serde(default)]
    pub coffee_dose_g: Option<f32>,
    #[serde(default)]
    pub total_water_ml: Option<f32>,
    #[serde(default)]
    pub target_duration_sec: Option<u32>,
    pub brewed_at: DateTime<Utc>,
    #[serde(default)]
    pub is_live: bool,
    #[serde(default)]
    pub accuracy_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BadgeMiniDto {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub rarity: String,
    #[serde(default)]
    pub rive_artboard: Option<String>,
}
