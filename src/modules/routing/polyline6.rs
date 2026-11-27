use std::collections::HashSet;

use crate::modules::beans_radar::spatial_math::haversine_meters;

const POLYLINE6_FACTOR: f64 = 1_000_000.0;

pub fn decode_polyline6(encoded: &str) -> Vec<[f64; 2]> {
    let bytes = encoded.as_bytes();
    let mut index = 0usize;
    let mut lat: i64 = 0;
    let mut lng: i64 = 0;
    let mut coords = Vec::new();

    while index < bytes.len() {
        let Some((dlat, next_idx)) = decode_signed_chunk(bytes, index) else {
            break;
        };
        index = next_idx;
        let Some((dlng, next_idx2)) = decode_signed_chunk(bytes, index) else {
            break;
        };
        index = next_idx2;
        lat += dlat;
        lng += dlng;
        coords.push([lat as f64 / POLYLINE6_FACTOR, lng as f64 / POLYLINE6_FACTOR]);
    }
    coords
}

fn decode_signed_chunk(bytes: &[u8], mut index: usize) -> Option<(i64, usize)> {
    let mut result: i64 = 0;
    let mut shift: u32 = 0;
    while index < bytes.len() {
        let b = (bytes[index] as i64) - 63;
        index += 1;
        if b < 0 || shift > 56 {
            return None;
        }
        result |= (b & 0x1f) << shift;
        shift += 5;
        if b < 0x20 {
            let delta = if (result & 1) != 0 {
                !(result >> 1)
            } else {
                result >> 1
            };
            return Some((delta, index));
        }
    }
    None
}

pub fn encode_polyline6(coords: &[[f64; 2]]) -> String {
    let mut out = String::new();
    let mut prev_lat: i64 = 0;
    let mut prev_lng: i64 = 0;

    for pt in coords {
        let lat = (pt[0] * POLYLINE6_FACTOR).round() as i64;
        let lng = (pt[1] * POLYLINE6_FACTOR).round() as i64;
        encode_signed(lat - prev_lat, &mut out);
        encode_signed(lng - prev_lng, &mut out);
        prev_lat = lat;
        prev_lng = lng;
    }
    out
}

fn encode_signed(value: i64, out: &mut String) {
    let mut v = if value < 0 { !(value << 1) } else { value << 1 };
    while v >= 0x20 {
        let ch = ((0x20 | (v & 0x1f)) + 63) as u8;
        out.push(ch as char);
        v >>= 5;
    }
    out.push(((v + 63) as u8) as char);
}

pub fn simplify_stride(coords: &[[f64; 2]], max_pts: usize) -> Vec<[f64; 2]> {
    if coords.len() <= max_pts || max_pts < 2 {
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

pub fn bearing_deg(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let p1 = lat1.to_radians();
    let p2 = lat2.to_radians();
    let dl = (lng2 - lng1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

pub fn polyline_length_meters(coords: &[[f64; 2]]) -> f64 {
    coords
        .windows(2)
        .map(|w| haversine_meters(w[0][0], w[0][1], w[1][0], w[1][1]))
        .sum()
}

/// Mengukur rasio kemiripan spasial dua jalur (grid ~45m) agar rute alternatif benar-benar berbeda.
pub fn jaccard_overlap_ratio(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let to_cell = |p: &[f64; 2]| -> (i64, i64) {
        ((p[0] * 2200.0).round() as i64, (p[1] * 2200.0).round() as i64)
    };
    let set_a: HashSet<(i64, i64)> = a.iter().map(to_cell).collect();
    let set_b: HashSet<(i64, i64)> = b.iter().map(to_cell).collect();
    if set_a.is_empty() {
        return 0.0;
    }
    let inter = set_a.intersection(&set_b).count() as f64;
    inter / set_a.len() as f64
}
