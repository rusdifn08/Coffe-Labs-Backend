use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use uuid::Uuid;

pub const PROFILE_CACHE_MAX_CAPACITY: u64 = 1_000;
pub const PROFILE_CACHE_TTL_SECS: u64 = 3;
pub const CHAT_MEMBER_CACHE_MAX_CAPACITY: u64 = 5_000;
pub const CHAT_MEMBER_CACHE_TTL_SECS: u64 = 300;
pub const FCM_TOKEN_CACHE_MAX_CAPACITY: u64 = 2;
pub const FCM_TOKEN_CACHE_TTL_SECS: u64 = 55 * 60;

/// Kumpulan L1 RAM Cache (`moka::future::Cache`) dengan kapasitas terbatas ketat
/// agar pemakaian memori proses tetap aman di bawah kuota 512 MB Render Free Tier.
#[derive(Clone)]
pub struct AppCaches {
    pub profile_l1: Cache<Uuid, Arc<serde_json::Value>>,
    pub chat_member_l1: Cache<(Uuid, Uuid), bool>,
    pub fcm_token_l1: Cache<String, String>,
}

impl Default for AppCaches {
    fn default() -> Self {
        Self::new()
    }
}

impl AppCaches {
    pub fn new() -> Self {
        Self {
            profile_l1: Self::build_profile_l1(),
            chat_member_l1: Self::build_chat_member_l1(),
            fcm_token_l1: Self::build_fcm_token_l1(),
        }
    }

    pub fn build_profile_l1() -> Cache<Uuid, Arc<serde_json::Value>> {
        Cache::builder()
            .max_capacity(PROFILE_CACHE_MAX_CAPACITY)
            .time_to_live(Duration::from_secs(PROFILE_CACHE_TTL_SECS))
            .build()
    }

    pub fn build_chat_member_l1() -> Cache<(Uuid, Uuid), bool> {
        Cache::builder()
            .max_capacity(CHAT_MEMBER_CACHE_MAX_CAPACITY)
            .time_to_live(Duration::from_secs(CHAT_MEMBER_CACHE_TTL_SECS))
            .build()
    }

    pub fn build_fcm_token_l1() -> Cache<String, String> {
        Cache::builder()
            .max_capacity(FCM_TOKEN_CACHE_MAX_CAPACITY)
            .time_to_live(Duration::from_secs(FCM_TOKEN_CACHE_TTL_SECS))
            .build()
    }

    pub fn entry_counts(&self) -> (u64, u64, u64) {
        (
            self.profile_l1.entry_count(),
            self.chat_member_l1.entry_count(),
            self.fcm_token_l1.entry_count(),
        )
    }
}
