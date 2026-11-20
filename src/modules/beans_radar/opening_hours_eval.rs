use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use serde_json::Value;

use crate::modules::beans_radar::dto::OpenStatusDto;

fn weekday_key(wd: Weekday) -> &'static str {
    match wd {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

fn next_weekday_key(wd: Weekday) -> &'static str {
    weekday_key(wd.succ())
}

fn tz_short_label(tz_name: &str) -> &'static str {
    match tz_name {
        "Asia/Makassar" => "WITA",
        "Asia/Jayapura" => "WIT",
        _ => "WIB",
    }
}

fn parse_hhmm_minutes(raw: &str) -> Option<i32> {
    let clean = raw.trim().replace('.', ":");
    let t = NaiveTime::parse_from_str(&clean, "%H:%M").ok()?;
    Some((t.hour() * 60 + t.minute()) as i32)
}

fn parse_slots(hours_json: &Value, key: &str) -> Vec<(i32, i32, String, String)> {
    let mut result = Vec::new();
    let Some(arr) = hours_json.get(key).and_then(|v| v.as_array()) else {
        return result;
    };
    for pair in arr {
        let Some(items) = pair.as_array() else {
            continue;
        };
        if items.len() < 2 {
            continue;
        }
        let (Some(start_str), Some(end_str)) = (items[0].as_str(), items[1].as_str()) else {
            continue;
        };
        if let (Some(s_min), Some(e_min)) = (parse_hhmm_minutes(start_str), parse_hhmm_minutes(end_str)) {
            result.push((s_min, e_min, start_str.to_string(), end_str.to_string()));
        }
    }
    result
}

/// Mengevaluasi status buka/tutup secara tepat di zona waktu WIB / WITA / WIT.
pub fn evaluate_open_status_at(
    opening_hours_json: &Value,
    is_24_hours: bool,
    timezone_name: &str,
    now_utc: DateTime<Utc>,
) -> OpenStatusDto {
    let tz_label = tz_short_label(timezone_name);
    if is_24_hours {
        return OpenStatusDto {
            is_open_now: true,
            is_closing_soon: false,
            status_label_id: format!("Buka 24 Jam ({})", tz_label),
            timezone_abbr: tz_label.to_string(),
        };
    }

    let tz: Tz = timezone_name.parse().unwrap_or(chrono_tz::Asia::Jakarta);
    let local_dt = now_utc.with_timezone(&tz);
    let now_min = (local_dt.hour() * 60 + local_dt.minute()) as i32;
    let today_key = weekday_key(local_dt.weekday());
    let today_slots = parse_slots(opening_hours_json, today_key);

    if today_slots.is_empty() {
        return OpenStatusDto {
            is_open_now: false,
            is_closing_soon: false,
            status_label_id: format!("Jam buka belum tersedia ({})", tz_label),
            timezone_abbr: tz_label.to_string(),
        };
    }

    for (start_min, end_min, _s_str, e_str) in &today_slots {
        let is_overnight = *end_min <= *start_min;
        let in_window = if is_overnight {
            now_min >= *start_min || now_min < *end_min
        } else {
            now_min >= *start_min && now_min < *end_min
        };
        if in_window {
            let remaining = if is_overnight && now_min >= *start_min {
                (1440 - now_min) + *end_min
            } else {
                *end_min - now_min
            };
            let closing_soon = remaining <= 45;
            let status_label = if closing_soon {
                format!("Segera Tutup • {} {}", e_str, tz_label)
            } else {
                format!("Buka • Tutup {} {}", e_str, tz_label)
            };
            return OpenStatusDto {
                is_open_now: true,
                is_closing_soon: closing_soon,
                status_label_id: status_label,
                timezone_abbr: tz_label.to_string(),
            };
        }
    }

    // Cek jadwal buka berikutnya hari ini
    if let Some((_, _, next_start, _)) = today_slots.iter().find(|(s, _, _, _)| *s > now_min) {
        return OpenStatusDto {
            is_open_now: false,
            is_closing_soon: false,
            status_label_id: format!("Tutup • Buka {} {}", next_start, tz_label),
            timezone_abbr: tz_label.to_string(),
        };
    }

    // Fallback ke jadwal buka besok
    let tomorrow_slots = parse_slots(opening_hours_json, next_weekday_key(local_dt.weekday()));
    let next_open = tomorrow_slots
        .first()
        .map(|(_, _, s, _)| format!("Tutup • Buka besok {} {}", s, tz_label))
        .unwrap_or_else(|| format!("Tutup hari ini ({})", tz_label));

    OpenStatusDto {
        is_open_now: false,
        is_closing_soon: false,
        status_label_id: next_open,
        timezone_abbr: tz_label.to_string(),
    }
}

pub fn evaluate_open_status_now(
    opening_hours_json: &Value,
    is_24_hours: bool,
    timezone_name: &str,
) -> OpenStatusDto {
    evaluate_open_status_at(opening_hours_json, is_24_hours, timezone_name, Utc::now())
}
