use roaring::RoaringBitmap;
use std::io::Cursor;

use crate::modules::rbac::models::AccountTier;

pub const DAILY_XP_CAP_TRIAL: i32 = 1_500;
pub const DAILY_XP_CAP_PRO: i32 = 4_500;
pub const DAILY_XP_CAP_BUSINESS: i32 = 10_000;

/// Kurva Level Eksponensial Terukur: XP = floor(100 * (level ^ 1.65))
/// Level 1 mulai dari 0 XP; Level 2 = 313 XP; Level 10 = 4,466 XP; Level 100 = 199,526 XP
pub fn xp_for_level(level: u32) -> i64 {
    if level <= 1 {
        return 0;
    }
    let l = level as f64;
    (100.0 * l.powf(1.65)).floor() as i64
}

pub fn level_from_total_xp(total_xp: i64) -> u32 {
    if total_xp <= 0 {
        return 1;
    }
    let mut low = 1u32;
    let mut high = 100u32;
    let mut best = 1u32;
    while low <= high {
        let mid = (low + high) / 2;
        if xp_for_level(mid) <= total_xp {
            best = mid;
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    best.clamp(1, 100)
}

pub fn tier_xp_multiplier(tier: AccountTier) -> f64 {
    match tier {
        AccountTier::Trial => 1.0,
        AccountTier::Pro => 1.5,
        AccountTier::Business => 2.0,
    }
}

pub fn daily_xp_cap_for_tier(tier: AccountTier) -> i32 {
    match tier {
        AccountTier::Trial => DAILY_XP_CAP_TRIAL,
        AccountTier::Pro => DAILY_XP_CAP_PRO,
        AccountTier::Business => DAILY_XP_CAP_BUSINESS,
    }
}

pub fn default_freezes_for_tier(tier: AccountTier) -> i32 {
    match tier {
        AccountTier::Trial => 0,
        AccountTier::Pro => 2,
        AccountTier::Business => 5,
    }
}

/// 20 Gelar Kehormatan Spesifik Dunia Kopi Specialty (Level 1 s/d Level 100)
pub fn level_title_for(level: u32) -> &'static str {
    match level {
        1..=4 => "Home Brewer Apprentice",
        5..=9 => "Paper Filter Initiate",
        10..=14 => "Gooseneck Pour Tactician",
        15..=19 => "Bloom & Degassing Adept",
        20..=24 => "Single-Origin Explorer",
        25..=29 => "Grind Micron Calibrator",
        30..=34 => "V60 Spiral Specialist",
        35..=39 => "Thermal Extraction Chemist",
        40..=44 => "Tetsu 4:6 Balance Artisan",
        45..=49 => "Sensory Cupping Lexiconist",
        50..=54 => "Anaerobic Fermentation Maven",
        55..=59 => "Osmotic Flow Virtuoso",
        60..=64 => "Water Mineral Architect",
        65..=69 => "Refractometer TDS Alchemist",
        70..=74 => "Roastery Head Dial-In Lead",
        75..=79 => "Geisha Varietal Custodian",
        80..=84 => "National Brewers Cup Finalist",
        85..=89 => "Hybrid Switch & Percolation Master",
        90..=99 => "Specialty Coffee Sovereign",
        _ => "WBrC World Grandmaster",
    }
}

pub fn decode_badges_bitmap(bytes: &[u8]) -> RoaringBitmap {
    if bytes.is_empty() {
        return RoaringBitmap::new();
    }
    RoaringBitmap::deserialize_from(Cursor::new(bytes)).unwrap_or_default()
}

pub fn encode_badges_bitmap(bitmap: &RoaringBitmap) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    if bitmap.serialize_into(&mut buf).is_ok() {
        buf
    } else {
        Vec::new()
    }
}
