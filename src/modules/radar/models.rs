use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RadarPlaceKind {
    Cafe,
    BeanSeller,
    RoasteryAndCafe,
}

impl RadarPlaceKind {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Cafe => "CAFE",
            Self::BeanSeller => "BEAN_SELLER",
            Self::RoasteryAndCafe => "ROASTERY_AND_CAFE",
        }
    }

    pub fn parse_opt(raw: Option<&str>) -> Option<Self> {
        match raw.map(|s| s.trim().to_uppercase()).as_deref() {
            Some("CAFE") => Some(Self::Cafe),
            Some("BEAN_SELLER") | Some("BEANS") => Some(Self::BeanSeller),
            Some("ROASTERY_AND_CAFE") | Some("ROASTERY") => Some(Self::RoasteryAndCafe),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadarScopeMode {
    All,
    Cafe,
    Beans,
}

impl RadarScopeMode {
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.map(|s| s.trim().to_uppercase()).as_deref() {
            Some("CAFE") | Some("CAFE_RADAR") => Self::Cafe,
            Some("BEANS") | Some("BEAN_SELLER") | Some("BEANS_RADAR") => Self::Beans,
            _ => Self::All,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyRadarPlaceDto {
    pub id: Uuid,
    pub linked_roastery_id: Option<Uuid>,
    pub name: String,
    pub kind: String,
    pub category_tag: String,
    pub address: String,
    pub district: String,
    pub city: String,
    pub province: String,
    pub timezone: String,
    pub latitude: f64,
    pub longitude: f64,
    pub distance_meters: f64,
    pub eta_walking_min: i32,
    pub eta_driving_min: i32,
    pub rating: f32,
    pub reviews_count: i32,
    pub sells_beans: bool,
    pub serves_cafe: bool,
    pub featured_beans: Value,
    pub brewing_methods: Value,
    pub phone: Option<String>,
    pub whatsapp_number: Option<String>,
    pub instagram_handle: Option<String>,
    pub opening_hours: Value,
    pub image_url: Option<String>,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStepDto {
    pub instruction: String,
    pub distance_meters: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePolylineDto {
    pub provider: String,
    pub mode: String,
    pub cache_hit: bool,
    pub distance_meters: f64,
    pub duration_seconds: f64,
    pub origin: [f64; 2],
    pub destination: [f64; 2],
    /// Koordinat polyline `[lat, lng]` siap dirender oleh `flutter_map` `PolylineLayer`.
    pub coordinates: Vec<[f64; 2]>,
    pub steps: Vec<RouteStepDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarStatsDto {
    pub total_places: i64,
    pub cafe_radar_total: i64,
    pub beans_radar_total: i64,
    pub dual_roastery_cafe_total: i64,
    pub cities_count: i64,
    pub provinces_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportResultDto {
    pub upserted_count: usize,
    pub skipped_invalid_count: usize,
}
