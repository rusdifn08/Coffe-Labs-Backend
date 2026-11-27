use std::time::Duration;
use serde_json::{json, Value};

use crate::modules::routing::{
    dto::{ModeRoutesBundleDto, RouteAlternativeDto, TravelMode, TurnStepDto},
    indonesia_costing::{
        calibrate_indonesian_eta, format_indonesian_maneuver, indonesia_timezone_info,
        MAX_ALTERNATIVE_ETA_RATIO, MAX_ALTERNATIVE_OVERLAP,
    },
    polyline6::{decode_polyline6, encode_polyline6, jaccard_overlap_ratio, simplify_stride},
};

const DEFAULT_VALHALLA_URL: &str = "https://valhalla1.openstreetmap.de/route";

pub async fn query_valhalla_mode(
    valhalla_url: Option<&str>,
    origin: [f64; 2],
    dest: [f64; 2],
    mode: TravelMode,
    avoid_tolls: bool,
    bearing_deg: Option<f64>,
    max_alternates: u8,
) -> Option<ModeRoutesBundleDto> {
    let endpoint = valhalla_url
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_VALHALLA_URL);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(7))
        .user_agent("CoffeeLabsRadar/3.0")
        .build()
        .ok()?;

    let origin_loc = if let Some(h) = bearing_deg.filter(|v| v.is_finite()) {
        json!({"lat": origin[0], "lon": origin[1], "heading": h, "heading_tolerance": 45})
    } else {
        json!({"lat": origin[0], "lon": origin[1]})
    };
    let (costing, options) = build_costing_payload(mode, avoid_tolls);
    let payload = json!({
        "locations": [origin_loc, {"lat": dest[0], "lon": dest[1]}],
        "costing": costing,
        "costing_options": options,
        "alternates": max_alternates.min(2),
        "units": "kilometers",
        "language": "id-ID"
    });

    let resp = client.post(endpoint).json(&payload).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: Value = resp.json().await.ok()?;
    parse_valhalla_bundle(&body, origin, mode, avoid_tolls)
}

fn build_costing_payload(mode: TravelMode, avoid_tolls: bool) -> (&'static str, Value) {
    match mode {
        TravelMode::Foot => (
            "pedestrian",
            json!({ "pedestrian": {
                "walking_speed": 4.8,
                "walkway_factor": 0.85,
                "alley_factor": 0.9,
                "use_roads": 0.35
            }}),
        ),
        TravelMode::Motorcycle => (
            "motorcycle",
            json!({ "motorcycle": {
                "use_tolls": 0.0,
                "use_highways": 0.2,
                "use_trails": 0.3,
                "use_tracks": 0.25,
                "use_living_streets": 0.9
            }}),
        ),
        TravelMode::Car => (
            "auto",
            json!({ "auto": {
                "use_tolls": if avoid_tolls { 0.0 } else { 1.0 },
                "alley_penalty": 120.0,
                "use_living_streets": 0.1
            }}),
        ),
    }
}

fn parse_valhalla_bundle(
    body: &Value,
    origin: [f64; 2],
    mode: TravelMode,
    avoid_tolls: bool,
) -> Option<ModeRoutesBundleDto> {
    let primary_trip = body.get("trip")?;
    let primary = parse_trip(primary_trip, origin, mode, avoid_tolls, 0, "Rute Terbaik")?;
    let mut alternatives = Vec::new();
    if let Some(alt_arr) = body.get("alternates").and_then(Value::as_array) {
        for (idx, alt) in alt_arr.iter().enumerate() {
            let Some(trip) = alt.get("trip") else { continue };
            let label = format!("Alternatif {}", idx + 1);
            let Some(dto) = parse_trip(trip, origin, mode, avoid_tolls, idx + 1, &label) else {
                continue;
            };
            let overlap = jaccard_overlap_ratio(&primary.coordinates, &dto.coordinates);
            let eta_ok = dto.duration_seconds <= primary.duration_seconds * MAX_ALTERNATIVE_ETA_RATIO;
            if overlap <= MAX_ALTERNATIVE_OVERLAP && eta_ok {
                alternatives.push(dto);
            }
        }
    }
    Some(ModeRoutesBundleDto { mode, primary, alternatives })
}

fn parse_trip(
    trip: &Value,
    origin: [f64; 2],
    mode: TravelMode,
    avoid_tolls: bool,
    idx: usize,
    label: &str,
) -> Option<RouteAlternativeDto> {
    let summary = trip.get("summary")?;
    let dist_m = summary.get("length")?.as_f64()? * 1000.0;
    let raw_dur = summary.get("time")?.as_f64()?;
    let has_toll = mode == TravelMode::Car
        && !avoid_tolls
        && summary.get("has_toll").and_then(Value::as_bool).unwrap_or(false);
    let leg = trip.get("legs")?.as_array()?.first()?;
    let shape = leg.get("shape")?.as_str()?;
    let full_coords = decode_polyline6(shape);
    if full_coords.len() < 2 {
        return None;
    }

    let steps = extract_maneuvers(leg, &full_coords);
    let (_, is_rush, is_day) = indonesia_timezone_info(origin[1]);
    let (dur_s, mult) = calibrate_indonesian_eta(
        mode, raw_dur, dist_m, has_toll, is_rush, is_day, steps.len(),
    );
    let sim = simplify_stride(&full_coords, 280);
    let poly6 = encode_polyline6(&sim);
    let uses_alleys = mode != TravelMode::Car;

    Some(RouteAlternativeDto {
        route_id: format!("valhalla_{}_{idx}", mode.cache_key_part()),
        label: label.to_string(),
        provider: "valhalla".to_string(),
        mode,
        cache_hit: false,
        distance_meters: (dist_m * 10.0).round() / 10.0,
        duration_seconds: dur_s,
        base_duration_seconds: raw_dur.round(),
        traffic_multiplier: mult,
        has_toll,
        avoided_tolls: avoid_tolls || mode != TravelMode::Car,
        uses_alleys,
        mode_badge: mode.default_badge(avoid_tolls).to_string(),
        polyline6: poly6,
        coordinates: sim,
        steps,
    })
}

fn extract_maneuvers(leg: &Value, coords: &[[f64; 2]]) -> Vec<TurnStepDto> {
    let mut steps = Vec::new();
    let Some(arr) = leg.get("maneuvers").and_then(Value::as_array) else {
        return steps;
    };
    for m in arr.iter().take(28) {
        let street = m
            .get("street_names")
            .and_then(Value::as_array)
            .and_then(|s| s.first())
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mtype_num = m.get("type").and_then(Value::as_u64).unwrap_or(8);
        let fallback_instr = format_indonesian_maneuver(&mtype_num.to_string(), "", &street);
        let raw_instr = m.get("instruction").and_then(Value::as_str).unwrap_or("").trim();
        let instruction = if raw_instr.is_empty() || mtype_num <= 6 {
            fallback_instr
        } else {
            raw_instr.to_string()
        };
        let pt_idx = m.get("begin_shape_index").and_then(Value::as_u64).unwrap_or(0) as usize;
        let pt = coords.get(pt_idx).or_else(|| coords.first()).copied().unwrap_or([0.0, 0.0]);
        steps.push(TurnStepDto {
            instruction,
            distance_meters: m.get("length").and_then(Value::as_f64).unwrap_or(0.0) * 1000.0,
            duration_seconds: m.get("time").and_then(Value::as_f64).unwrap_or(0.0),
            maneuver_type: mtype_num.to_string(),
            street_name: street,
            lat: pt[0],
            lng: pt[1],
        });
    }
    steps
}
