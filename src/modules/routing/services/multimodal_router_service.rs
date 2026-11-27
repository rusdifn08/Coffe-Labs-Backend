use std::sync::Arc;
use h3o::{LatLng, Resolution};
use sqlx::{PgPool, Row};

use crate::{
    core::{redis::{redis_get_bounded, redis_set_ex_bounded}, AppState},
    modules::{
        beans_radar::spatial_math::haversine_meters,
        routing::{
            dto::{CompareModesResponseDto, ModeRoutesBundleDto, ReverseGeocodeResponseDto, TravelMode},
            indonesia_costing::indonesia_timezone_info,
            services::{osrm_multimodal_adapter, rust_local_astar_solver, valhalla_adapter},
        },
    },
};

const ROUTE_REDIS_TTL_SECS: i64 = 180;

pub fn build_h3_route_key(
    mode: TravelMode, avoid_tolls: bool, origin: [f64; 2], dest: [f64; 2],
) -> String {
    let h3_cell = |lat: f64, lng: f64| -> String {
        LatLng::new(lat, lng)
            .map(|p| p.to_cell(Resolution::Ten).to_string())
            .unwrap_or_else(|_| format!("{lat:.4}_{lng:.4}"))
    };
    let o_cell = h3_cell(origin[0], origin[1]);
    let d_cell = h3_cell(dest[0], dest[1]);
    let toll_flag = if mode == TravelMode::Car && avoid_tolls { "notoll" } else { "std" };
    format!("route:v3:{}:{toll_flag}:{o_cell}:{d_cell}", mode.cache_key_part())
}

pub async fn resolve_mode_bundle(
    state: &AppState,
    origin: [f64; 2],
    dest: [f64; 2],
    mode: TravelMode,
    avoid_tolls: bool,
    bearing_deg: Option<f64>,
    max_alternates: u8,
) -> ModeRoutesBundleDto {
    let cache_key = build_h3_route_key(mode, avoid_tolls, origin, dest);
    if bearing_deg.is_none() {
        if let Some(cached_val) = state.caches.route_l1.get(&cache_key).await {
            if let Ok(mut b) = serde_json::from_value::<ModeRoutesBundleDto>((*cached_val).clone()) {
                mark_cache_hit(&mut b);
                return b;
            }
        }
        if let Some(pool) = state.redis.active_pool() {
            if let Some(raw) = redis_get_bounded(&pool, &cache_key).await {
                if let Ok(mut b) = serde_json::from_str::<ModeRoutesBundleDto>(&raw) {
                    mark_cache_hit(&mut b);
                    return b;
                }
            }
        }
    }

    let bundle = match valhalla_adapter::query_valhalla_mode(
        state.config.valhalla_url.as_deref(),
        origin, dest, mode, avoid_tolls, bearing_deg, max_alternates,
    ).await {
        Some(v) => v,
        None => match osrm_multimodal_adapter::query_secondary_profiles(
            state.config.geoapify_api_key.as_deref(), origin, dest, mode, avoid_tolls,
        ).await {
            Some(s) => s,
            None => rust_local_astar_solver::solve_local_multimodal_astar(origin, dest, mode, avoid_tolls),
        },
    };

    if bearing_deg.is_none() {
        if let Ok(val) = serde_json::to_value(&bundle) {
            state.caches.route_l1.insert(cache_key.clone(), Arc::new(val)).await;
        }
        if let Some(pool) = state.redis.active_pool() {
            if let Ok(raw) = serde_json::to_string(&bundle) {
                redis_set_ex_bounded(&pool, &cache_key, raw, ROUTE_REDIS_TTL_SECS).await;
            }
        }
    }
    bundle
}

pub async fn compare_all_modes(
    state: &AppState,
    origin: [f64; 2],
    dest: [f64; 2],
    avoid_tolls: bool,
    alternates: u8,
) -> CompareModesResponseDto {
    let (foot, motorcycle, car) = tokio::join!(
        resolve_mode_bundle(state, origin, dest, TravelMode::Foot, true, None, alternates),
        resolve_mode_bundle(state, origin, dest, TravelMode::Motorcycle, true, None, alternates),
        resolve_mode_bundle(state, origin, dest, TravelMode::Car, avoid_tolls, None, alternates),
    );
    let (tz_abbr, is_rush, _) = indonesia_timezone_info(origin[1]);
    CompareModesResponseDto {
        origin, destination: dest,
        timezone_abbr: tz_abbr.to_string(),
        is_rush_hour: is_rush,
        avoid_tolls,
        foot, motorcycle, car,
    }
}

pub async fn resolve_reverse_geocode(
    state: &AppState, lat: f64, lng: f64,
) -> ReverseGeocodeResponseDto {
    if let Some((pid, dist)) = state.caches.beans_radar_rtree.nearest_place_id_within_radius(lat, lng, 5_000.0) {
        if let Some(row) = state.caches.radar_place_l1.get(&pid).await {
            if let Some(dto) = format_place_geo(&row.district_kecamatan, &row.city_kabupaten, &row.province, dist, "L1_RTREE") {
                return dto;
            }
        }
    }
    if let Some(dto) = query_db_nearest_area(&state.pool, lat, lng).await {
        return dto;
    }
    nearest_indonesia_city_fallback(lat, lng)
}

async fn query_db_nearest_area(pool: &PgPool, lat: f64, lng: f64) -> Option<ReverseGeocodeResponseDto> {
    if let Ok(Some(row)) = sqlx::query(
        r#"SELECT district, city, province,
                  ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) AS dist_m
           FROM radar_places WHERE is_active = TRUE AND COALESCE(city, '') <> ''
           ORDER BY location <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography LIMIT 1"#,
    ).bind(lat).bind(lng).fetch_optional(pool).await {
        let dist_m: f64 = row.try_get("dist_m").unwrap_or(99_999.0);
        if dist_m <= 25_000.0 {
            let d: String = row.try_get("district").unwrap_or_default();
            let c: String = row.try_get("city").unwrap_or_default();
            let p: String = row.try_get("province").unwrap_or_default();
            if let Some(dto) = format_place_geo(&d, &c, &p, dist_m, "POSTGIS_NEAREST") { return Some(dto); }
        }
    }
    let row = sqlx::query(
        r#"SELECT district_kecamatan AS district, city_kabupaten AS city, province,
                  ST_Distance(location, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) AS dist_m
           FROM coffee_radar_places WHERE is_active = TRUE AND COALESCE(city_kabupaten, '') <> ''
           ORDER BY location <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography LIMIT 1"#,
    ).bind(lat).bind(lng).fetch_optional(pool).await.ok()??;
    let dist_m: f64 = row.try_get("dist_m").unwrap_or(99_999.0);
    if dist_m > 25_000.0 { return None; }
    let d: String = row.try_get("district").unwrap_or_default();
    let c: String = row.try_get("city").unwrap_or_default();
    let p: String = row.try_get("province").unwrap_or_default();
    format_place_geo(&d, &c, &p, dist_m, "POSTGIS_COFFEE_NEAREST")
}

fn format_place_geo(dist: &str, city: &str, prov: &str, dist_m: f64, src: &str) -> Option<ReverseGeocodeResponseDto> {
    let d = dist.trim();
    let c = city.trim();
    if d.is_empty() && c.is_empty() { return None; }
    let label = match (d.is_empty(), c.is_empty()) {
        (false, false) => format!("{d}, {c}"),
        (true, false) => c.to_string(),
        _ => d.to_string(),
    };
    Some(ReverseGeocodeResponseDto {
        district: d.to_string(), city: c.to_string(), province: prov.trim().to_string(),
        label, distance_meters: (dist_m * 10.0).round() / 10.0, source: src.to_string(),
    })
}

pub fn nearest_indonesia_city_fallback(lat: f64, lng: f64) -> ReverseGeocodeResponseDto {
    const HUBS: &[(&str, &str, f64, f64)] = &[
        ("Gedebage", "Bandung", -6.9565, 107.6942), ("Coblong", "Bandung", -6.895, 107.615),
        ("Gambir", "Jakarta Pusat", -6.175, 106.827), ("Kebayoran Baru", "Jakarta Selatan", -6.244, 106.801),
        ("Gubeng", "Surabaya", -7.265, 112.752), ("Gondokusuman", "Yogyakarta", -7.783, 110.375),
        ("Semarang Tengah", "Semarang", -6.982, 110.415), ("Kuta Utara", "Bali", -8.647, 115.138),
        ("Medan Barat", "Medan", 3.595, 98.672), ("Ujung Pandang", "Makassar", -5.147, 119.423),
        ("Ilir Timur", "Palembang", -2.976, 104.775),
    ];
    let mut best = ("Indonesia", "Indonesia", f64::MAX);
    for &(dist, city, hlat, hlng) in HUBS {
        let d = haversine_meters(lat, lng, hlat, hlng);
        if d < best.2 { best = (dist, city, d); }
    }
    let label = if best.2 <= 45_000.0 { format!("{}, {}", best.0, best.1) } else { "Lokasi Anda".to_string() };
    ReverseGeocodeResponseDto {
        district: best.0.to_string(), city: best.1.to_string(), province: "Indonesia".to_string(),
        label, distance_meters: (best.2 * 10.0).round() / 10.0, source: "GEO_HUB_FALLBACK".to_string(),
    }
}

fn mark_cache_hit(bundle: &mut ModeRoutesBundleDto) {
    bundle.primary.cache_hit = true;
    for alt in &mut bundle.alternatives { alt.cache_hit = true; }
}
