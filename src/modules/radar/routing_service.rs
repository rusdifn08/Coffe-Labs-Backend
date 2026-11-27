use std::time::Duration;

use serde_json::Value;

use crate::{
    core::{
        error::{AppError, AppResult},
        redis::{redis_get_bounded, redis_set_ex_bounded, RedisManager},
    },
    modules::radar::{
        models::RoutePolylineDto,
        route_parsers::{parse_geoapify_route, parse_osrm_route},
    },
};

pub const ROUTE_CACHE_TTL_SECS: i64 = 21_600; // 6 jam

/// Bulatkan koordinat ke 3 angka desimal (~100 meter grid) untuk menghemat kuota Redis & Geoapify.
pub fn round_grid_3(val: f64) -> f64 {
    (val * 1000.0).round() / 1000.0
}

pub fn build_route_cache_key(mode: &str, origin_lat: f64, origin_lng: f64, dest_key: &str) -> String {
    let olat = round_grid_3(origin_lat);
    let olng = round_grid_3(origin_lng);
    format!("radar:route:{mode}:{olat:.3}:{olng:.3}:{dest_key}")
}

pub async fn resolve_route_with_cache(
    redis: &RedisManager,
    geoapify_api_key: Option<&str>,
    origin: [f64; 2],
    destination: [f64; 2],
    dest_key: &str,
    mode: &str,
) -> AppResult<RoutePolylineDto> {
    let cache_key = build_route_cache_key(mode, origin[0], origin[1], dest_key);
    if let Some(pool) = redis.active_pool() {
        if let Some(raw) = redis_get_bounded(&pool, &cache_key).await {
            if let Ok(mut cached) = serde_json::from_str::<RoutePolylineDto>(&raw) {
                cached.cache_hit = true;
                return Ok(cached);
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("CoffeeLabsRadar/3.0")
        .build()
        .map_err(|e| AppError::Internal(format!("Gagal membangun HTTP client rute: {e}")))?;

    let mut result: Option<RoutePolylineDto> = None;

    if let Some(api_key) = geoapify_api_key.filter(|k| !k.trim().is_empty()) {
        let url = format!(
            "https://api.geoapify.com/v1/routing?waypoints={:.6},{:.6}|{:.6},{:.6}&mode={}&apiKey={}",
            origin[0], origin[1], destination[0], destination[1], mode, api_key.trim()
        );
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    result = parse_geoapify_route(&body, mode, origin, destination);
                }
            }
        }
    }

    if result.is_none() {
        let osrm_base = match mode {
            "walk" => "https://routing.openstreetmap.de/routed-foot/route/v1/foot",
            "motorcycle" => "https://routing.openstreetmap.de/routed-bike/route/v1/driving",
            _ => "https://routing.openstreetmap.de/routed-car/route/v1/driving",
        };
        let url = format!(
            "{osrm_base}/{:.6},{:.6};{:.6},{:.6}?overview=full&geometries=geojson&steps=true",
            origin[1], origin[0], destination[1], destination[0]
        );
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(body) = resp.json::<Value>().await {
                    result = parse_osrm_route(&body, mode, origin, destination);
                }
            }
        }
    }

    let dto = result.ok_or_else(|| {
        AppError::Internal("Layanan routing jalan sedang sibuk. Silakan coba sesaat lagi.".into())
    })?;

    if let Some(pool) = redis.active_pool() {
        if let Ok(serialized) = serde_json::to_string(&dto) {
            redis_set_ex_bounded(&pool, &cache_key, serialized, ROUTE_CACHE_TTL_SECS).await;
        }
    }
    Ok(dto)
}
