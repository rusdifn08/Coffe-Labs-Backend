use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TravelMode {
    Foot,
    Motorcycle,
    Car,
}

impl TravelMode {
    pub fn parse_loose(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "walk" | "foot" | "walking" | "pedestrian" | "jalan" => Self::Foot,
            "drive" | "car" | "driving" | "auto" | "mobil" => Self::Car,
            _ => Self::Motorcycle,
        }
    }

    pub fn api_slug(self) -> &'static str {
        match self {
            Self::Foot => "walk",
            Self::Motorcycle => "motorcycle",
            Self::Car => "drive",
        }
    }

    pub fn cache_key_part(self) -> &'static str {
        match self {
            Self::Foot => "foot",
            Self::Motorcycle => "moto",
            Self::Car => "car",
        }
    }

    pub fn max_speed_mps(self) -> f64 {
        match self {
            Self::Foot => 2.2,
            Self::Motorcycle => 22.0,
            Self::Car => 30.0,
        }
    }

    pub fn default_badge(self, avoid_tolls: bool) -> &'static str {
        match self {
            Self::Foot => "Trotoar & Gang • Bebas Macet",
            Self::Motorcycle => "Bebas Tol • Lewat Gang Motor",
            Self::Car if avoid_tolls => "Non-Tol • Jalur Arteri Mobil",
            Self::Car => "Jalur Cepat Mobil (Boleh Tol)",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompareModesRequest {
    pub origin_lat: f64,
    pub origin_lng: f64,
    pub dest_lat: f64,
    pub dest_lng: f64,
    pub avoid_tolls: Option<bool>,
    pub alternates: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NavigateRequest {
    pub origin_lat: f64,
    pub origin_lng: f64,
    pub dest_lat: f64,
    pub dest_lng: f64,
    pub mode: TravelMode,
    pub avoid_tolls: Option<bool>,
    pub bearing_deg: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReverseGeocodeQuery {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStepDto {
    pub instruction: String,
    pub distance_meters: f64,
    pub duration_seconds: f64,
    pub maneuver_type: String,
    pub street_name: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteAlternativeDto {
    pub route_id: String,
    pub label: String,
    pub provider: String,
    pub mode: TravelMode,
    pub cache_hit: bool,
    pub distance_meters: f64,
    pub duration_seconds: f64,
    pub base_duration_seconds: f64,
    pub traffic_multiplier: f64,
    pub has_toll: bool,
    pub avoided_tolls: bool,
    pub uses_alleys: bool,
    pub mode_badge: String,
    pub polyline6: String,
    pub coordinates: Vec<[f64; 2]>,
    pub steps: Vec<TurnStepDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeRoutesBundleDto {
    pub mode: TravelMode,
    pub primary: RouteAlternativeDto,
    pub alternatives: Vec<RouteAlternativeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareModesResponseDto {
    pub origin: [f64; 2],
    pub destination: [f64; 2],
    pub timezone_abbr: String,
    pub is_rush_hour: bool,
    pub avoid_tolls: bool,
    pub foot: ModeRoutesBundleDto,
    pub motorcycle: ModeRoutesBundleDto,
    pub car: ModeRoutesBundleDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseGeocodeResponseDto {
    pub district: String,
    pub city: String,
    pub province: String,
    pub label: String,
    pub distance_meters: f64,
    pub source: String,
}
