use serde_json::Value;

use crate::modules::radar::models::{RoutePolylineDto, RouteStepDto};

pub fn simplify_stride(coords: &[[f64; 2]], max_pts: usize) -> Vec<[f64; 2]> {
    if coords.len() <= max_pts {
        return coords.to_vec();
    }
    let step = (coords.len() as f64 / max_pts as f64).ceil().max(1.0) as usize;
    let mut out: Vec<[f64; 2]> = coords.iter().step_by(step).copied().collect();
    if let (Some(last_src), Some(last_out)) = (coords.last(), out.last()) {
        if last_src != last_out {
            out.push(*last_src);
        }
    }
    out
}

/// Ekstrak koordinat `[lat, lng]`, jarak, durasi, dan langkah belokan dari payload Geoapify GeoJSON.
pub fn parse_geoapify_route(
    body: &Value,
    mode: &str,
    origin: [f64; 2],
    destination: [f64; 2],
) -> Option<RoutePolylineDto> {
    let feat = body.get("features")?.as_array()?.first()?;
    let props = feat.get("properties")?;
    let distance_meters = props.get("distance")?.as_f64().unwrap_or(0.0);
    let duration_seconds = props.get("time")?.as_f64().unwrap_or(0.0);

    let mut coords: Vec<[f64; 2]> = Vec::new();
    let geom = feat.get("geometry")?;
    let raw_coords = geom.get("coordinates")?.as_array()?;
    let geom_type = geom.get("type").and_then(Value::as_str).unwrap_or("");

    if geom_type == "MultiLineString" {
        for segment in raw_coords {
            if let Some(pts) = segment.as_array() {
                for pt in pts {
                    if let (Some(lng), Some(lat)) = (
                        pt.get(0).and_then(Value::as_f64),
                        pt.get(1).and_then(Value::as_f64),
                    ) {
                        coords.push([lat, lng]);
                    }
                }
            }
        }
    } else {
        for pt in raw_coords {
            if let (Some(lng), Some(lat)) = (
                pt.get(0).and_then(Value::as_f64),
                pt.get(1).and_then(Value::as_f64),
            ) {
                coords.push([lat, lng]);
            }
        }
    }

    if coords.len() < 2 {
        return None;
    }

    let mut steps = Vec::new();
    if let Some(legs) = props.get("legs").and_then(Value::as_array) {
        for leg in legs {
            if let Some(raw_steps) = leg.get("steps").and_then(Value::as_array) {
                for st in raw_steps.iter().take(25) {
                    let instr = st
                        .get("instruction")
                        .and_then(|i| i.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or("Ikuti jalur jalan utama")
                        .to_string();
                    steps.push(RouteStepDto {
                        instruction: instr,
                        distance_meters: st.get("distance").and_then(Value::as_f64).unwrap_or(0.0),
                        duration_seconds: st.get("time").and_then(Value::as_f64).unwrap_or(0.0),
                    });
                }
            }
        }
    }

    Some(RoutePolylineDto {
        provider: "geoapify".to_string(),
        mode: mode.to_string(),
        cache_hit: false,
        distance_meters,
        duration_seconds,
        origin,
        destination,
        coordinates: simplify_stride(&coords, 260),
        steps,
    })
}

/// Ekstrak koordinat `[lat, lng]` dari respons OSRM GeoJSON (cadangan gratis 0 API key).
pub fn parse_osrm_route(
    body: &Value,
    mode: &str,
    origin: [f64; 2],
    destination: [f64; 2],
) -> Option<RoutePolylineDto> {
    let route = body.get("routes")?.as_array()?.first()?;
    let distance_meters = route.get("distance")?.as_f64().unwrap_or(0.0);
    let raw_dur = route.get("duration")?.as_f64().unwrap_or(0.0);
    let duration_seconds = if mode == "motorcycle" {
        (raw_dur * 0.82).max(60.0)
    } else {
        raw_dur
    };

    let raw_coords = route.get("geometry")?.get("coordinates")?.as_array()?;
    let mut coords = Vec::with_capacity(raw_coords.len());
    for pt in raw_coords {
        if let (Some(lng), Some(lat)) = (
            pt.get(0).and_then(Value::as_f64),
            pt.get(1).and_then(Value::as_f64),
        ) {
            coords.push([lat, lng]);
        }
    }
    if coords.len() < 2 {
        return None;
    }

    let mut steps = Vec::new();
    if let Some(legs) = route.get("legs").and_then(Value::as_array) {
        for leg in legs {
            if let Some(raw_steps) = leg.get("steps").and_then(Value::as_array) {
                for st in raw_steps.iter().take(25) {
                    let street = st.get("name").and_then(Value::as_str).unwrap_or("").trim();
                    let mtype = st
                        .get("maneuver")
                        .and_then(|m| m.get("type"))
                        .and_then(Value::as_str)
                        .unwrap_or("continue");
                    let instr = if street.is_empty() {
                        format!("Lanjutkan perjalanan ({mtype})")
                    } else {
                        format!("Menuju {street}")
                    };
                    steps.push(RouteStepDto {
                        instruction: instr,
                        distance_meters: st.get("distance").and_then(Value::as_f64).unwrap_or(0.0),
                        duration_seconds: st.get("duration").and_then(Value::as_f64).unwrap_or(0.0),
                    });
                }
            }
        }
    }

    Some(RoutePolylineDto {
        provider: "osrm".to_string(),
        mode: mode.to_string(),
        cache_hit: false,
        distance_meters,
        duration_seconds,
        origin,
        destination,
        coordinates: simplify_stride(&coords, 260),
        steps,
    })
}
