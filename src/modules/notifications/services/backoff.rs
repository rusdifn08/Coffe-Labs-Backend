use chrono::{DateTime, FixedOffset, Timelike, Utc};
use rand::Rng;

use crate::modules::notifications::{
    events::NotificationCategory, models::NotificationPreferenceRow,
};

pub const T_BASE_MS: u64 = 1_000;
pub const T_MAX_MS: u64 = 60_000;
pub const R_MAX: u32 = 5;

/// Menghitung jeda Exponential Backoff + Full Jitter (AWSEncoded):
/// `sleep = rand(0, min(T_max, T_base * 2^attempt))`, sambil menghormati `Retry-After` bila ada.
pub fn compute_full_jitter_delay_ms(attempt: u32, retry_after_secs: Option<u64>) -> u64 {
    let shift = attempt.min(6);
    let cap = T_BASE_MS.saturating_mul(1_u64 << shift).min(T_MAX_MS);
    let mut rng = rand::thread_rng();
    let jitter = rng.gen_range(0..=cap);
    match retry_after_secs {
        Some(secs) => jitter.max(secs.saturating_mul(1_000)).min(T_MAX_MS),
        None => jitter,
    }
}

pub fn resolve_timezone_offset_secs(tz: &str) -> i32 {
    match tz.trim() {
        "Asia/Jakarta" | "WIB" | "Asia/Pontianak" => 7 * 3600,
        "Asia/Makassar" | "WITA" | "Asia/Singapore" | "Asia/Kuala_Lumpur" => 8 * 3600,
        "Asia/Jayapura" | "WIT" | "Asia/Tokyo" => 9 * 3600,
        "UTC" | "Etc/UTC" | "GMT" => 0,
        _ => 7 * 3600,
    }
}

/// Mengevaluasi apakah waktu lokal pengguna sedang berada di dalam jendela Jam Hening (Quiet Hours),
/// termasuk jendela yang melintasi tengah malam (misal 22:00 - 07:00).
pub fn is_quiet_hours_now(pref: &NotificationPreferenceRow, utc_now: DateTime<Utc>) -> bool {
    let (Some(start), Some(end)) = (pref.quiet_hours_start_minute, pref.quiet_hours_end_minute)
    else {
        return false;
    };
    if !(0..1440).contains(&start) || !(0..1440).contains(&end) || start == end {
        return false;
    }
    let offset_secs = resolve_timezone_offset_secs(&pref.timezone);
    let Some(fixed) = FixedOffset::east_opt(offset_secs) else {
        return false;
    };
    let local_time = utc_now.with_timezone(&fixed);
    let current_minute = (local_time.hour() * 60 + local_time.minute()) as i32;

    if start < end {
        current_minute >= start && current_minute < end
    } else {
        current_minute >= start || current_minute < end
    }
}

pub fn is_event_enabled_for_user(
    pref: &NotificationPreferenceRow,
    category: NotificationCategory,
    event_type: &str,
) -> bool {
    match category {
        NotificationCategory::Chat => pref.enable_chat_messages,
        NotificationCategory::SocialGraph => pref.enable_follow_follback,
        NotificationCategory::ThreadActivity => {
            if event_type.contains("LIKE") || event_type.contains("RECOOK") {
                pref.enable_likes_and_recipe_tries
            } else {
                pref.enable_thread_replies
            }
        }
        NotificationCategory::Gamification => pref.enable_live_brew_and_streak,
        NotificationCategory::Billing => pref.enable_transactional_billing,
    }
}
