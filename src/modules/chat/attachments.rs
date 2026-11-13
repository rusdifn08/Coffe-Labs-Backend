use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    Text,
    Image,
    RecipeCard,
    BeanCard,
    BrewLog,
}

impl MessageType {
    pub fn as_db_str(self) -> &'static str {
        match self {
            MessageType::Text => "TEXT",
            MessageType::Image => "IMAGE",
            MessageType::RecipeCard => "RECIPE_CARD",
            MessageType::BeanCard => "BEAN_CARD",
            MessageType::BrewLog => "BREW_LOG",
        }
    }

    pub fn from_db_str(raw: &str) -> Self {
        match raw {
            "IMAGE" => MessageType::Image,
            "RECIPE_CARD" => MessageType::RecipeCard,
            "BEAN_CARD" => MessageType::BeanCard,
            "BREW_LOG" => MessageType::BrewLog,
            _ => MessageType::Text,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptStatus {
    Sent,
    Delivered,
    Read,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InteractiveAttachment {
    V60RecipeCard {
        recipe_id: String,
        title: String,
        brewer_name: String,
        dose_grams: f32,
        water_ml: f32,
        temp_c: u16,
        target_duration_sec: u32,
        #[serde(default)]
        steps_preview: Vec<String>,
    },
    CoffeeBeanCard {
        #[serde(default)]
        bean_id: Option<String>,
        roastery: String,
        bean_name: String,
        origin: String,
        process: String,
        #[serde(default)]
        tasting_notes: Vec<String>,
        #[serde(default)]
        image_url: Option<String>,
    },
    BrewImageMedia {
        image_url: String,
        width: u32,
        height: u32,
        #[serde(default)]
        blurhash: Option<String>,
        #[serde(default)]
        caption: Option<String>,
    },
    BrewSessionResult {
        title: String,
        method: String,
        accuracy_score: u16,
        xp_earned: u32,
        #[serde(default)]
        brew_duration_sec: Option<u32>,
        #[serde(default)]
        notes: Option<String>,
    },
}
