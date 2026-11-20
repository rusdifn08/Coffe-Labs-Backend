use serde::Serialize;

/// Deterministic level curve matching Coffee Labs Home Screen:
/// `level_start_xp(L) = floor(150 * (L - 1)^1.8)`
#[inline]
pub fn level_start_xp(level: i32) -> i64 {
    if level <= 1 {
        return 0;
    }
    (150.0 * ((level - 1) as f64).powf(1.8)).floor() as i64
}

pub fn compute_rank_title(level: i32) -> &'static str {
    match level {
        1 => "Home Brewer",
        2 => "Pour-Over Apprentice",
        3 => "Extraction Explorer",
        4 => "Dial-In Alchemist",
        5 => "Sensory Artisan",
        6..=9 => "Brewing Specialist",
        10..=19 => "Master Pour-Over",
        _ => "WBrC Grandmaster",
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LevelSnapshot {
    pub level: i32,
    pub rank_title: &'static str,
    pub level_start_xp: i64,
    pub next_level_xp: i64,
    pub progress_ratio: f64,
}

pub fn compute_level_snapshot(total_xp: i64) -> LevelSnapshot {
    let xp = total_xp.max(0);
    let mut level = 1i32;
    while level < 99 && xp >= level_start_xp(level + 1) {
        level += 1;
    }
    let start = level_start_xp(level);
    let next = level_start_xp(level + 1);
    let span = (next - start).max(1) as f64;
    let ratio = ((xp - start) as f64 / span).clamp(0.0, 1.0);
    LevelSnapshot {
        level,
        rank_title: compute_rank_title(level),
        level_start_xp: start,
        next_level_xp: next,
        progress_ratio: ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_curve_1635_xp_matches_lv4_dial_in_alchemist() {
        let snap = compute_level_snapshot(1635);
        assert_eq!(snap.level, 4);
        assert_eq!(snap.rank_title, "Dial-In Alchemist");
        assert_eq!(snap.level_start_xp, 1083);
        assert_eq!(snap.next_level_xp, 1818);
        assert!((snap.progress_ratio - 0.7497).abs() < 0.005);
    }

    #[test]
    fn test_level_up_overflow_lv4_to_lv5() {
        let before = compute_level_snapshot(1635);
        let after = compute_level_snapshot(1835);
        assert_eq!(before.level, 4);
        assert_eq!(after.level, 5);
        assert_eq!(after.rank_title, "Sensory Artisan");
        assert!(after.progress_ratio > 0.0 && after.progress_ratio < 0.05);
    }
}
