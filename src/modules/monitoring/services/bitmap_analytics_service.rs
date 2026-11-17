use std::collections::HashMap;
use std::io::Cursor;

use chrono::{Duration, NaiveDate};
use dashmap::{DashMap, DashSet};
use roaring::RoaringBitmap;

use crate::modules::monitoring::{
    models::{FeatureAdoptionStat, RetentionCohortRow},
    services::dossier_aggregator::{feature_human_label, normalize_feature_bucket},
};

const MAX_RAM_DAYS: i64 = 35;
const TRACKED_FEATURES: [&str; 8] = [
    "brew_timer", "social_coffee", "coffee_club", "recipe_lab",
    "ai_scanner", "gamification", "marketplace", "business_pos",
];

/// Mesin DAU/WAU/MAU, Retention Cohort & Feature Adoption berbasis RoaringBitmap (max 35 hari di RAM).
pub struct BitmapAnalyticsService {
    bitmaps: DashMap<(NaiveDate, String), RoaringBitmap>,
    dirty_keys: DashSet<(NaiveDate, String)>,
}

impl Default for BitmapAnalyticsService {
    fn default() -> Self { Self::new() }
}

impl BitmapAnalyticsService {
    pub fn new() -> Self {
        Self { bitmaps: DashMap::new(), dirty_keys: DashSet::new() }
    }

    pub fn entry_count(&self) -> usize { self.bitmaps.len() }

    pub fn load_bitmap_bytes(&self, date: NaiveDate, key: String, bytes: &[u8]) {
        if bytes.is_empty() { return; }
        if let Ok(bm) = RoaringBitmap::deserialize_from(Cursor::new(bytes)) {
            self.bitmaps.insert((date, key), bm);
        }
    }

    pub fn record_user_activity(&self, date: NaiveDate, seq_id: u32, feature_bucket: &str, tier: &str, is_aod: bool) {
        self.insert_bit(date, "active_user", seq_id);
        let bucket = normalize_feature_bucket(feature_bucket);
        if bucket != "general" { self.insert_bit(date, &format!("feature_{bucket}"), seq_id); }
        if is_aod || bucket == "aod_kiosk" { self.insert_bit(date, "aod_active", seq_id); }
        let tier_key = match tier.trim().to_ascii_lowercase().as_str() {
            "business" => "tier_business",
            "pro" => "tier_pro",
            _ => "tier_trial",
        };
        self.insert_bit(date, tier_key, seq_id);
    }

    fn insert_bit(&self, date: NaiveDate, key: &str, seq_id: u32) {
        let map_key = (date, key.to_string());
        let mut entry = self.bitmaps.entry(map_key.clone()).or_default();
        if entry.insert(seq_id) { self.dirty_keys.insert(map_key); }
    }

    pub fn drain_dirty_snapshots(&self) -> Vec<(NaiveDate, String, Vec<u8>, i32)> {
        let keys: Vec<(NaiveDate, String)> = self.dirty_keys.iter().map(|k| k.clone()).collect();
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            self.dirty_keys.remove(&key);
            if let Some(bm) = self.bitmaps.get(&key) {
                let mut bytes = Vec::with_capacity(64);
                if bm.serialize_into(&mut bytes).is_ok() {
                    out.push((key.0, key.1, bytes, i32::try_from(bm.len()).unwrap_or(i32::MAX)));
                }
            }
        }
        out
    }

    pub fn evict_old_days(&self, today: NaiveDate) {
        let cutoff = today - Duration::days(MAX_RAM_DAYS);
        self.bitmaps.retain(|(d, _), _| *d >= cutoff);
        self.dirty_keys.retain(|(d, _)| *d >= cutoff);
    }

    pub fn union_range(&self, end_date: NaiveDate, days: i64, metric_key: &str) -> RoaringBitmap {
        let mut out = RoaringBitmap::new();
        for offset in 0..days {
            if let Some(bm) = self.bitmaps.get(&(end_date - Duration::days(offset), metric_key.to_string())) {
                out |= bm.value();
            }
        }
        out
    }

    pub fn compute_dau_wau_mau(&self, today: NaiveDate) -> (u64, u64, u64, f64) {
        let dau = self.bitmaps.get(&(today, "active_user".to_string())).map(|bm| bm.len()).unwrap_or(0);
        let wau = self.union_range(today, 7, "active_user").len();
        let mau = self.union_range(today, 30, "active_user").len();
        let stickiness = if mau > 0 { ((dau as f64 / mau as f64) * 1000.0).round() / 10.0 } else { 0.0 };
        (dau, wau, mau, stickiness)
    }

    pub fn compute_feature_adoption_7d(&self, today: NaiveDate, mins: &HashMap<String, i64>) -> Vec<FeatureAdoptionStat> {
        let active_7d = self.union_range(today, 7, "active_user");
        let denom = active_7d.len().max(1);
        let mut stats: Vec<FeatureAdoptionStat> = TRACKED_FEATURES.iter().map(|&fk| {
            let feat_7d = self.union_range(today, 7, &format!("feature_{fk}"));
            let adopted = if active_7d.is_empty() { 0 } else { (&active_7d & &feat_7d).len() };
            let pct = if active_7d.is_empty() { 0.0 } else { ((adopted as f64 / denom as f64) * 1000.0).round() / 10.0 };
            FeatureAdoptionStat {
                feature_key: fk.to_string(),
                label: feature_human_label(fk).to_string(),
                active_users_7d: adopted,
                adoption_percent: pct,
                total_minutes_7d: *mins.get(fk).unwrap_or(&0),
            }
        }).collect();
        stats.sort_by(|a, b| b.adoption_percent.total_cmp(&a.adoption_percent));
        stats
    }

    pub fn compute_retention_cohorts(&self, today: NaiveDate) -> Vec<RetentionCohortRow> {
        let mut rows = Vec::with_capacity(6);
        for days_ago in [1_i64, 3, 7, 14, 21, 30] {
            let cohort_date = today - Duration::days(days_ago);
            let Some(base) = self.bitmaps.get(&(cohort_date, "active_user".to_string())) else { continue };
            let size = base.len();
            if size == 0 { continue; }
            let pct_at = |offset: i64| -> Option<f64> {
                let target = cohort_date + Duration::days(offset);
                if target > today { return None; }
                let ret = self.bitmaps.get(&(target, "active_user".to_string())).map(|bm| (base.value() & bm.value()).len()).unwrap_or(0);
                Some(((ret as f64 / size as f64) * 1000.0).round() / 10.0)
            };
            rows.push(RetentionCohortRow {
                cohort_date, cohort_size: size,
                d1_pct: pct_at(1), d3_pct: pct_at(3), d7_pct: pct_at(7), d14_pct: pct_at(14), d30_pct: pct_at(30),
            });
        }
        rows
    }
}
