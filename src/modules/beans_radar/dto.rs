use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct NearbyQueryParams {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub radius_km: Option<f64>,
    pub category: Option<String>,
    pub open_now: Option<bool>,
    pub min_rating: Option<f32>,
    pub sells_beans: Option<bool>,
    pub sort: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ViewportQueryParams {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
    pub category: Option<String>,
    pub open_now: Option<bool>,
    pub min_rating: Option<f32>,
    pub sells_beans: Option<bool>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQueryParams {
    pub q: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenStatusDto {
    pub is_open_now: bool,
    pub is_closing_soon: bool,
    pub status_label_id: String,
    pub timezone_abbr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarPlaceCardDto {
    pub id: Uuid,
    pub slug: String,
    pub linked_roastery_id: Option<Uuid>,
    pub name: String,
    pub category: String,
    pub sub_categories: Vec<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub h3_res7: String,
    pub full_address: String,
    pub district_kecamatan: String,
    pub city_kabupaten: String,
    pub province: String,
    pub rating: f32,
    pub reviews_count: i32,
    pub bayesian_score: f32,
    pub price_level: i16,
    pub distance_meters: f64,
    pub distance_label: String,
    pub walk_eta_min: Option<i32>,
    pub ride_eta_min: i32,
    pub open_status: OpenStatusDto,
    pub sells_roast_beans: bool,
    pub has_manual_brew_v60: bool,
    pub featured_bean_summary: Option<String>,
    pub google_maps_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_verified_partner: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarLineupPreviewDto {
    pub id: Uuid,
    pub name: String,
    pub origin: String,
    pub roast_profile: String,
    pub process: String,
    pub tasting_notes: String,
    pub price_100g_label: String,
    pub price_200g_label: String,
    pub bag_image_url: Option<String>,
    pub badge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarPlaceDetailDto {
    pub card: RadarPlaceCardDto,
    pub timezone: String,
    pub is_24_hours: bool,
    pub opening_hours: Value,
    pub phone_number: Option<String>,
    pub whatsapp_number: Option<String>,
    pub instagram_handle: Option<String>,
    pub website_url: Option<String>,
    pub is_favorite: bool,
    pub linked_roastery_lineups: Vec<RadarLineupPreviewDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyRadarResponseDto {
    pub center_lat: f64,
    pub center_lng: f64,
    pub radius_km: f64,
    pub query_h3_res7: String,
    pub engine_latency_us: u64,
    pub cache_tier: String,
    pub total_indexed_places: usize,
    pub total_matched: usize,
    pub items: Vec<RadarPlaceCardDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarCategoryCountDto {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarCityCountDto {
    pub city: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarStatsDto {
    pub total_active_places: i64,
    pub total_cities: i64,
    pub verified_partners_count: i64,
    pub rtree_loaded_nodes: usize,
    pub by_category: Vec<RadarCategoryCountDto>,
    pub top_cities: Vec<RadarCityCountDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleFavoriteResponseDto {
    pub place_id: Uuid,
    pub is_favorite: bool,
}
