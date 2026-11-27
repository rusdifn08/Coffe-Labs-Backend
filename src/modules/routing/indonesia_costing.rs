use chrono::{Timelike, Utc};
use chrono_tz::Asia::{Jakarta, Jayapura, Makassar};

use crate::modules::routing::dto::TravelMode;

pub const MAX_ALTERNATIVE_OVERLAP: f64 = 0.72;
pub const MAX_ALTERNATIVE_ETA_RATIO: f64 = 1.35;

/// Menghitung zona waktu lokal Indonesia (WIB/WITA/WIT) dan status jam sibuk.
pub fn indonesia_timezone_info(lng: f64) -> (&'static str, bool, bool) {
    let now_utc = Utc::now();
    let (abbr, hour, minute) = if lng >= 134.0 {
        let local = now_utc.with_timezone(&Jayapura);
        ("WIT", local.hour(), local.minute())
    } else if lng >= 114.5 {
        let local = now_utc.with_timezone(&Makassar);
        ("WITA", local.hour(), local.minute())
    } else {
        let local = now_utc.with_timezone(&Jakarta);
        ("WIB", local.hour(), local.minute())
    };
    let mins = hour * 60 + minute;
    let is_rush = (420..=570).contains(&mins) || (990..=1170).contains(&mins);
    let is_day = (360..=1260).contains(&mins);
    (abbr, is_rush, is_day)
}

/// Pengali kecepatan jam sibuk Indonesia: Mobil terdampak macet berat (0.55x), Motor selap-selip (0.82x), Kaki bebas macet (1.0x).
pub fn traffic_speed_factor(mode: TravelMode, is_rush_hour: bool, is_daytime: bool) -> f64 {
    match mode {
        TravelMode::Foot => 1.0,
        TravelMode::Motorcycle => {
            if is_rush_hour {
                0.82
            } else if is_daytime {
                0.92
            } else {
                1.0
            }
        }
        TravelMode::Car => {
            if is_rush_hour {
                0.55
            } else if is_daytime {
                0.76
            } else {
                0.95
            }
        }
    }
}

/// Tobler's Hiking Function (km/jam) berdasarkan kemiringan lereng `slope = dz / length`.
pub fn tobler_speed_kmh(slope: f64) -> f64 {
    (6.0 * (-3.5 * (slope + 0.05).abs()).exp()).clamp(1.8, 6.2)
}

/// Mengkalibrasi ETA nyata perkotaan Indonesia + penalti belok/lampu merah left-hand traffic.
pub fn calibrate_indonesian_eta(
    mode: TravelMode,
    raw_duration_s: f64,
    distance_m: f64,
    has_toll: bool,
    is_rush_hour: bool,
    is_daytime: bool,
    turn_count: usize,
) -> (f64, f64) {
    let base_raw = if raw_duration_s > 5.0 {
        raw_duration_s
    } else {
        distance_m / mode.max_speed_mps().max(1.2)
    };
    let speed_factor = if mode == TravelMode::Car && has_toll {
        (traffic_speed_factor(mode, is_rush_hour, is_daytime) + 0.22).min(0.96)
    } else {
        traffic_speed_factor(mode, is_rush_hour, is_daytime)
    };

    let (adjusted, reported_mult) = match mode {
        TravelMode::Foot => {
            let walk_mps = tobler_speed_kmh(0.015) / 3.6;
            let tobler_sec = (distance_m / walk_mps).max(30.0);
            let crossing_penalty = (turn_count as f64) * 2.5;
            (((base_raw * 0.55) + (tobler_sec * 0.45) + crossing_penalty).max(30.0), 1.0)
        }
        TravelMode::Motorcycle => {
            let congestion_adjusted = base_raw / speed_factor;
            // Keunggulan lane-filtering perkotaan motor + penalti belok ringan (6 detik / manuver)
            let moto_s = (congestion_adjusted * 0.86) + (turn_count as f64) * 3.0;
            (moto_s.clamp(35.0, 86_400.0), speed_factor)
        }
        TravelMode::Car => {
            let congestion_adjusted = base_raw / speed_factor;
            // Left-Hand Traffic (Indonesia): penalti potong arus kanan + antrean lampu merah (2.2x motor)
            let lht_turn_penalty = (turn_count as f64) * 7.5;
            ((congestion_adjusted + lht_turn_penalty).clamp(45.0, 86_400.0), speed_factor)
        }
    };
    (adjusted.round(), reported_mult)
}

pub fn format_indonesian_maneuver(
    maneuver_type: &str,
    modifier: &str,
    street: &str,
) -> String {
    let clean = street.trim();
    let road = if clean.is_empty() {
        String::new()
    } else {
        format!(" ke {clean}")
    };
    let mtype = maneuver_type.trim().to_ascii_lowercase();
    let mod_l = modifier.trim().to_ascii_lowercase();

    match mtype.as_str() {
        "depart" | "1" | "2" | "3" => {
            if clean.is_empty() {
                "Mulai perjalanan menuju lokasi tujuan".to_string()
            } else {
                format!("Mulai perjalanan dari {clean}")
            }
        }
        "arrive" | "4" | "5" | "6" => "Tiba di lokasi tujuan".to_string(),
        "roundabout" | "rotary" | "26" | "27" => format!("Masuk bundaran{road}"),
        _ => {
            if mod_l.contains("uturn") || mtype == "12" || mtype == "13" {
                format!("Putar balik{road}")
            } else if mod_l.contains("sharp left") || mtype == "14" {
                format!("Belok tajam ke kiri{road}")
            } else if mod_l.contains("slight left") || mtype == "16" {
                format!("Ambil jalur sedikit ke kiri{road}")
            } else if mod_l.contains("left") || mtype == "15" {
                format!("Belok kiri{road}")
            } else if mod_l.contains("sharp right") || mtype == "11" {
                format!("Belok tajam ke kanan{road}")
            } else if mod_l.contains("slight right") || mtype == "9" {
                format!("Ambil jalur sedikit ke kanan{road}")
            } else if mod_l.contains("right") || mtype == "10" {
                format!("Belok kanan{road}")
            } else if clean.is_empty() {
                "Lanjutkan perjalanan lurus".to_string()
            } else {
                format!("Lurus terus mengikuti {clean}")
            }
        }
    }
}
