use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CoffeeBeanCatalogRow {
    pub id: Uuid,
    pub roastery_id: Option<Uuid>,
    pub roastery_name: String,
    pub bean_name: String,
    pub origin_country: Option<String>,
    pub origin_region: Option<String>,
    pub variety: String,
    pub process: String,
    pub roast_level: String,
    pub altitude_masl: Option<String>,
    pub tasting_notes: Vec<String>,
    pub recommended_water_temp_c: Option<i32>,
    pub recommended_recipe_slugs: Vec<String>,
    pub packaging_image_url: Option<String>,
    pub search_vector_text: Option<String>,
    pub is_verified: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RoasteryCatalogRow {
    pub id: Uuid,
    pub name: String,
    pub normalized_aliases: Vec<String>,
    pub country: Option<String>,
    pub logo_url: Option<String>,
    pub is_verified: Option<bool>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLexiconData {
    pub roastery: Option<String>,
    pub bean_name: Option<String>,
    pub origin: Option<String>,
    pub variety: Option<String>,
    pub process: Option<String>,
    pub altitude: Option<String>,
    pub tasting_notes: Vec<String>,
}
