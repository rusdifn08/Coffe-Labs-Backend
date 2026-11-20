use chrono::{DateTime, Utc};
use rstar::{PointDistance, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

use crate::modules::beans_radar::spatial_math::to_unit_sphere;

/// Node ringan (~64 byte) pada L1 In-Memory R*-Tree untuk KNN < 0.08 ms.
#[derive(Debug, Clone)]
pub struct CompactSpatialNode {
    pub place_id: Uuid,
    pub point_3d: [f64; 3],
    pub latitude: f64,
    pub longitude: f64,
    pub category_code: u8,
    pub rating_x10: u8,
    #[allow(dead_code)]
    pub bayesian_x1000: u16,
    pub sells_roast_beans: bool,
    #[allow(dead_code)]
    pub is_verified_partner: bool,
}

impl CompactSpatialNode {
    pub fn category_to_code(cat: &str) -> u8 {
        match cat.trim().to_uppercase().as_str() {
            "ROASTERY" => 1,
            "SPECIALTY_CAFE" => 2,
            "COFFEE_SHOP" => 3,
            "BEAN_SELLER" => 4,
            "CAFE_RESTO" => 5,
            _ => 3,
        }
    }

    pub fn from_compact_row(row: &CompactNodeDbRow) -> Self {
        let code = Self::category_to_code(&row.category);
        Self {
            place_id: row.id,
            point_3d: to_unit_sphere(row.latitude, row.longitude),
            latitude: row.latitude,
            longitude: row.longitude,
            category_code: code,
            rating_x10: ((row.rating * 10.0).round().clamp(0.0, 50.0)) as u8,
            bayesian_x1000: ((row.bayesian_score * 1000.0).round().clamp(0.0, 5000.0)) as u16,
            sells_roast_beans: row.sells_roast_beans,
            is_verified_partner: row.is_verified_partner,
        }
    }
}

impl RTreeObject for CompactSpatialNode {
    type Envelope = AABB<[f64; 3]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.point_3d)
    }
}

impl PointDistance for CompactSpatialNode {
    fn distance_2(&self, point: &[f64; 3]) -> f64 {
        let dx = self.point_3d[0] - point[0];
        let dy = self.point_3d[1] - point[1];
        let dz = self.point_3d[2] - point[2];
        dx * dx + dy * dy + dz * dz
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct CompactNodeDbRow {
    pub id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub category: String,
    pub rating: f32,
    pub bayesian_score: f32,
    pub sells_roast_beans: bool,
    pub is_verified_partner: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CoffeeRadarPlaceRow {
    pub id: Uuid,
    pub external_source: String,
    pub linked_roastery_id: Option<Uuid>,
    pub slug: String,
    pub name: String,
    pub category: String,
    pub sub_categories: Value,
    pub latitude: f64,
    pub longitude: f64,
    pub h3_res7: i64,
    pub h3_res9: i64,
    pub full_address: String,
    pub district_kecamatan: String,
    pub city_kabupaten: String,
    pub province: String,
    pub timezone: String,
    pub rating: f32,
    pub reviews_count: i32,
    pub bayesian_score: f32,
    pub price_level: Option<i16>,
    pub opening_hours_json: Value,
    pub is_24_hours: bool,
    pub sells_roast_beans: bool,
    pub has_manual_brew_v60: bool,
    pub featured_bean_summary: Option<String>,
    pub phone_number: Option<String>,
    pub whatsapp_number: Option<String>,
    pub instagram_handle: Option<String>,
    pub website_url: Option<String>,
    pub google_maps_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_verified_partner: bool,
    pub last_verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RoasteryLineupPreviewRow {
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
