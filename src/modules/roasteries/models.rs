use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RoasteryRow {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub slug: String,
    pub name: String,
    pub city: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub cluster: String,
    pub description: String,
    pub specialties: String,
    pub sales_highlight: String,
    pub logo_url: String,
    pub logo_public_id: Option<String>,
    pub banner_url: Option<String>,
    pub banner_public_id: Option<String>,
    pub whatsapp_number: Option<String>,
    pub instagram_handle: Option<String>,
    pub website_url: Option<String>,
    pub is_verified_business: bool,
    pub is_published: bool,
    pub rating: f64,
    pub total_lineups_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct RoasteryLineupRow {
    pub id: Uuid,
    pub roastery_id: Uuid,
    pub name: String,
    pub origin: String,
    pub roast_profile: String,
    pub variety: String,
    pub process: String,
    pub tasting_notes: String,
    pub net_weight_grams: i32,
    pub price_idr: i64,
    pub price_100g_label: String,
    pub price_200g_label: String,
    pub original_package_label: String,
    pub bag_image_url: Option<String>,
    pub bag_image_public_id: Option<String>,
    pub badge: Option<String>,
    pub is_available: bool,
    pub display_order: i32,
    pub recommended_brew_method: Option<String>,
    pub recommended_temp_c: Option<i16>,
    pub recommended_ratio: Option<String>,
    pub recommended_grind_size: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoasteryWithLineupsResponse {
    #[serde(flatten)]
    pub roastery: RoasteryRow,
    pub lineups: Vec<RoasteryLineupRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteLineupOutcome {
    pub deleted_id: Uuid,
    pub roastery_id: Uuid,
    pub remaining_lineups_count: i32,
}
