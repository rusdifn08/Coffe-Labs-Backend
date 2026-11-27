use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
pub struct NearbyRadarQuery {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: Option<f64>,
    pub kind: Option<String>,
    #[serde(alias = "type", alias = "radar_mode", alias = "radar_scope")]
    pub mode: Option<String>,
    pub category: Option<String>,
    pub strict_category: Option<String>,
    pub min_rating: Option<f32>,
    pub search: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ViewportRadarQuery {
    pub north: f64,
    pub south: f64,
    pub east: f64,
    pub west: f64,
    pub kind: Option<String>,
    #[serde(alias = "type", alias = "radar_mode", alias = "radar_scope")]
    pub mode: Option<String>,
    pub category: Option<String>,
    pub strict_category: Option<String>,
    pub min_rating: Option<f32>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RouteQuery {
    pub origin_lat: Option<f64>,
    pub origin_lng: Option<f64>,
    /// Alternatif format `origin=lat,lng`
    pub origin: Option<String>,
    pub dest_place_id: Option<Uuid>,
    #[serde(alias = "dest_id")]
    pub dest_id: Option<Uuid>,
    pub dest_lat: Option<f64>,
    pub dest_lng: Option<f64>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportPlaceItem {
    pub external_place_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub category_tag: Option<String>,
    pub address: String,
    pub district: Option<String>,
    pub city: String,
    pub province: Option<String>,
    pub lat: f64,
    pub lng: f64,
    pub rating: Option<f32>,
    pub reviews_count: Option<i32>,
    pub featured_beans: Option<Value>,
    pub brewing_methods: Option<Value>,
    pub phone: Option<String>,
    pub instagram_handle: Option<String>,
    pub opening_hours: Option<Value>,
    pub image_url: Option<String>,
    pub is_verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BulkImportRequest {
    pub places: Vec<BulkImportPlaceItem>,
}

pub fn validate_coords(lat: f64, lng: f64) -> AppResult<(f64, f64)> {
    if !lat.is_finite() || !lng.is_finite() {
        return Err(AppError::Validation("Koordinat lat/lng harus berupa angka valid.".into()));
    }
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
        return Err(AppError::Validation("Koordinat lat/lng berada di luar batas geografis.".into()));
    }
    Ok((lat, lng))
}

pub fn validate_indonesia_coords(lat: f64, lng: f64) -> bool {
    lat.is_finite()
        && lng.is_finite()
        && (-11.5..=6.5).contains(&lat)
        && (94.5..=141.5).contains(&lng)
        && !(lat.abs() < 0.01 && lng.abs() < 0.01)
}

pub fn parse_origin_coords(q: &RouteQuery) -> AppResult<(f64, f64)> {
    if let (Some(lat), Some(lng)) = (q.origin_lat, q.origin_lng) {
        return validate_coords(lat, lng);
    }
    if let Some(raw) = q.origin.as_deref() {
        let mut parts = raw.split(',');
        let lat_s = parts
            .next()
            .ok_or_else(|| AppError::Validation("Format origin harus 'lat,lng'.".into()))?;
        let lng_s = parts
            .next()
            .ok_or_else(|| AppError::Validation("Format origin harus 'lat,lng'.".into()))?;
        let lat = lat_s
            .trim()
            .parse::<f64>()
            .map_err(|_| AppError::Validation("origin latitude tidak valid.".into()))?;
        let lng = lng_s
            .trim()
            .parse::<f64>()
            .map_err(|_| AppError::Validation("origin longitude tidak valid.".into()))?;
        return validate_coords(lat, lng);
    }
    Err(AppError::Validation(
        "Parameter origin_lat & origin_lng (atau origin=lat,lng) wajib diisi.".into(),
    ))
}

pub fn normalize_route_mode(raw: Option<&str>) -> &'static str {
    match raw.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("drive") | Some("car") | Some("driving") => "drive",
        Some("walk") | Some("foot") | Some("walking") => "walk",
        _ => "motorcycle",
    }
}
