use std::time::Duration;
use serde_json::Value;

use crate::modules::routing::{
    dto::{ModeRoutesBundleDto, RouteAlternativeDto, TravelMode, TurnStepDto},
    indonesia_costing::{
        calibrate_indonesian_eta, format_indonesian_maneuver, indonesia_timezone_info,
        MAX_ALTERNATIVE_ETA_RATIO, MAX_ALTERNATIVE_OVERLAP,
    },
    polyline6::{encode_polyline6, jaccard_overlap_ratio, simplify_stride},
};

pub async fn query_secondary_profiles(
    geoapify_key: Option<&str>, origin: [f64; 2], dest: [f64; 2], mode: TravelMode, avoid_tolls: bool,
) -> Option<ModeRoutesBundleDto> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(7)).user_agent("CoffeeLabsRadar/3.0").build().ok()?;
    if let Some(bundle) = query_fossgis_osrm(&client, origin, dest, mode, avoid_tolls).await {
        return Some(bundle);
    }
    if let Some(key) = geoapify_key.map(str::trim).filter(|k| !k.is_empty()) {
        return query_geoapify(&client, key, origin, dest, mode, avoid_tolls).await;
    }
    None
}

async fn query_fossgis_osrm(
    client: &reqwest::Client, origin: [f64; 2], dest: [f64; 2], mode: TravelMode, avoid_tolls: bool,
) -> Option<ModeRoutesBundleDto> {
    let base = match mode {
        TravelMode::Foot => "https://routing.openstreetmap.de/routed-foot/route/v1/foot",
        TravelMode::Motorcycle => "https://routing.openstreetmap.de/routed-bike/route/v1/driving",
        TravelMode::Car => "https://routing.openstreetmap.de/routed-car/route/v1/driving",
    };
    let exclude = if mode == TravelMode::Car && avoid_tolls { "&exclude=toll" } else { "" };
    let url = format!(
        "{base}/{:.6},{:.6};{:.6},{:.6}?overview=full&geometries=geojson&steps=true&alternatives=true{exclude}",
        origin[1], origin[0], dest[1], dest[0]
    );
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() { return None; }
    let body: Value = resp.json().await.ok()?;
    let routes = body.get("routes")?.as_array()?;
    let primary = parse_osrm_item(routes.first()?, origin, mode, avoid_tolls, 0, "Rute Terbaik")?;
    let mut alternatives = Vec::new();
    for (idx, r) in routes.iter().skip(1).enumerate().take(2) {
        let Some(dto) = parse_osrm_item(r, origin, mode, avoid_tolls, idx + 1, &format!("Alternatif {}", idx + 1)) else { continue };
        if jaccard_overlap_ratio(&primary.coordinates, &dto.coordinates) <= MAX_ALTERNATIVE_OVERLAP
            && dto.duration_seconds <= primary.duration_seconds * MAX_ALTERNATIVE_ETA_RATIO
        {
            alternatives.push(dto);
        }
    }
    Some(ModeRoutesBundleDto { mode, primary, alternatives })
}

fn parse_osrm_item(
    route: &Value, origin: [f64; 2], mode: TravelMode, avoid_tolls: bool, idx: usize, label: &str,
) -> Option<RouteAlternativeDto> {
    let dist_m = route.get("distance")?.as_f64()?;
    let raw_dur = route.get("duration")?.as_f64()?;
    let raw_coords = route.get("geometry")?.get("coordinates")?.as_array()?;
    let full_coords: Vec<[f64; 2]> = raw_coords.iter().filter_map(|pt| {
        Some([pt.get(1)?.as_f64()?, pt.get(0)?.as_f64()?])
    }).collect();
    if full_coords.len() < 2 { return None; }
    let steps = parse_osrm_steps(route, &full_coords);
    let has_toll = steps.iter().any(|s| s.street_name.to_ascii_lowercase().contains("tol"));
    let (_, is_rush, is_day) = indonesia_timezone_info(origin[1]);
    let (dur_s, mult) = calibrate_indonesian_eta(
        mode, raw_dur, dist_m, has_toll && !avoid_tolls, is_rush, is_day, steps.len(),
    );
    let sim = simplify_stride(&full_coords, 280);
    Some(RouteAlternativeDto {
        route_id: format!("osrm_de_{}_{idx}", mode.cache_key_part()),
        label: label.to_string(),
        provider: "osrm_fossgis".to_string(),
        mode,
        cache_hit: false,
        distance_meters: (dist_m * 10.0).round() / 10.0,
        duration_seconds: dur_s,
        base_duration_seconds: raw_dur.round(),
        traffic_multiplier: mult,
        has_toll: has_toll && !avoid_tolls && mode == TravelMode::Car,
        avoided_tolls: avoid_tolls || mode != TravelMode::Car,
        uses_alleys: mode != TravelMode::Car,
        mode_badge: mode.default_badge(avoid_tolls).to_string(),
        polyline6: encode_polyline6(&sim),
        coordinates: sim,
        steps,
    })
}

fn parse_osrm_steps(route: &Value, fallback_coords: &[[f64; 2]]) -> Vec<TurnStepDto> {
    let mut steps = Vec::new();
    let Some(legs) = route.get("legs").and_then(Value::as_array) else { return steps };
    let def = fallback_coords.first().copied().unwrap_or([0.0, 0.0]);
    for leg in legs {
        let Some(raw_steps) = leg.get("steps").and_then(Value::as_array) else { continue };
        for st in raw_steps.iter().take(28) {
            let street = st.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let man = st.get("maneuver");
            let mtype = man.and_then(|m| m.get("type")).and_then(Value::as_str).unwrap_or("continue");
            let mmod = man.and_then(|m| m.get("modifier")).and_then(Value::as_str).unwrap_or("");
            let loc = man.and_then(|m| m.get("location")).and_then(Value::as_array);
            steps.push(TurnStepDto {
                instruction: format_indonesian_maneuver(mtype, mmod, &street),
                distance_meters: st.get("distance").and_then(Value::as_f64).unwrap_or(0.0),
                duration_seconds: st.get("duration").and_then(Value::as_f64).unwrap_or(0.0),
                maneuver_type: mtype.to_string(),
                street_name: street,
                lat: loc.and_then(|a| a.get(1)).and_then(Value::as_f64).unwrap_or(def[0]),
                lng: loc.and_then(|a| a.first()).and_then(Value::as_f64).unwrap_or(def[1]),
            });
        }
    }
    steps
}

async fn query_geoapify(
    client: &reqwest::Client, key: &str, origin: [f64; 2], dest: [f64; 2], mode: TravelMode, avoid_tolls: bool,
) -> Option<ModeRoutesBundleDto> {
    let avoid = if mode == TravelMode::Car && avoid_tolls { "&avoid=tolls" } else { "" };
    let url = format!(
        "https://api.geoapify.com/v1/routing?waypoints={:.6},{:.6}|{:.6},{:.6}&mode={}{avoid}&apiKey={key}",
        origin[0], origin[1], dest[0], dest[1], mode.api_slug()
    );
    let body: Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let feat = body.get("features")?.as_array()?.first()?;
    let props = feat.get("properties")?;
    let dist_m = props.get("distance")?.as_f64()?;
    let raw_dur = props.get("time")?.as_f64()?;
    let pts = feat.get("geometry")?.get("coordinates")?.as_array()?;
    let mut coords = Vec::new();
    for seg in pts {
        if let Some(arr) = seg.as_array() {
            for pt in arr {
                if let (Some(lng), Some(lat)) = (pt.get(0).and_then(Value::as_f64), pt.get(1).and_then(Value::as_f64)) {
                    coords.push([lat, lng]);
                }
            }
        }
    }
    if coords.len() < 2 { return None; }
    let (_, is_rush, is_day) = indonesia_timezone_info(origin[1]);
    let (dur_s, mult) = calibrate_indonesian_eta(mode, raw_dur, dist_m, false, is_rush, is_day, 3);
    let sim = simplify_stride(&coords, 260);
    let primary = RouteAlternativeDto {
        route_id: format!("geoapify_{}_0", mode.cache_key_part()),
        label: "Rute Terbaik".to_string(),
        provider: "geoapify".to_string(),
        mode,
        cache_hit: false,
        distance_meters: (dist_m * 10.0).round() / 10.0,
        duration_seconds: dur_s,
        base_duration_seconds: raw_dur.round(),
        traffic_multiplier: mult,
        has_toll: false,
        avoided_tolls: avoid_tolls || mode != TravelMode::Car,
        uses_alleys: mode != TravelMode::Car,
        mode_badge: mode.default_badge(avoid_tolls).to_string(),
        polyline6: encode_polyline6(&sim),
        coordinates: sim,
        steps: Vec::new(),
    };
    Some(ModeRoutesBundleDto { mode, primary, alternatives: Vec::new() })
}
