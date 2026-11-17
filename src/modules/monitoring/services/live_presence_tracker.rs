use std::collections::HashMap;
use std::time::Duration;

use chrono::Utc;
use dashmap::DashMap;
use fred::{
    clients::RedisPool,
    interfaces::{KeysInterface, SortedSetsInterface},
    types::Expiration,
};
use uuid::Uuid;

use crate::modules::monitoring::models::LiveAccountState;

const LIVE_ZSET_KEY: &str = "mon:live_accounts";
const LIVE_STATE_PREFIX: &str = "mon:live_state:";
const LIVE_TTL_SECS: i64 = 90;
const REDIS_TIMEOUT_MS: u64 = 200;
const MAX_MEM_LIVE_ENTRIES: usize = 2_000;

/// Registry akun online real-time berbasis Redis ZSET + bounded DashMap fallback (aman < 1 MB RAM).
pub struct LivePresenceTracker {
    redis: Option<RedisPool>,
    mem_live: DashMap<Uuid, LiveAccountState>,
}

impl LivePresenceTracker {
    pub fn new(redis: Option<RedisPool>) -> Self {
        Self {
            redis,
            mem_live: DashMap::new(),
        }
    }

    pub fn mem_entry_count(&self) -> usize {
        self.mem_live.len()
    }

    pub async fn upsert_live_state(&self, state: LiveAccountState) {
        self.prune_if_needed();
        let uid = state.user_id;
        let epoch = state.last_seen_epoch;
        self.mem_live.insert(uid, state.clone());

        let Some(pool) = &self.redis else {
            return;
        };
        if let Ok(json) = serde_json::to_string(&state) {
            let key = format!("{LIVE_STATE_PREFIX}{uid}");
            let zset_fut = pool.zadd::<i64, _, _>(
                LIVE_ZSET_KEY,
                None,
                None,
                false,
                false,
                (epoch as f64, uid.to_string()),
            );
            let set_fut = pool.set::<(), _, _>(
                key,
                json,
                Some(Expiration::EX(LIVE_TTL_SECS)),
                None,
                false,
            );
            let _ = tokio::time::timeout(
                Duration::from_millis(REDIS_TIMEOUT_MS),
                async {
                    let _ = tokio::join!(zset_fut, set_fut);
                },
            )
            .await;
        }
    }

    /// Memperbarui sinyal online ringan saat WS chat Ping/SendMessage diterima.
    pub fn touch_ws_activity(&self, user_id: Uuid, tier: &str, route: &str) {
        let now_epoch = Utc::now().timestamp();
        self.prune_if_needed();
        self.mem_live
            .entry(user_id)
            .and_modify(|e| {
                e.last_seen_epoch = now_epoch;
                if e.current_route.is_empty() {
                    e.current_route = route.to_string();
                }
            })
            .or_insert_with(|| LiveAccountState {
                user_id,
                current_route: route.to_string(),
                feature_bucket: "social_coffee".to_string(),
                tier: tier.to_string(),
                platform: "mobile".to_string(),
                app_version: String::new(),
                is_brewing_now: false,
                last_seen_epoch: now_epoch,
            });
    }

    pub fn count_live_now(&self) -> usize {
        let cutoff = Utc::now().timestamp() - LIVE_TTL_SECS;
        self.mem_live
            .iter()
            .filter(|entry| entry.value().last_seen_epoch >= cutoff)
            .count()
    }

    pub fn sweep_expired(&self) -> Vec<Uuid> {
        let cutoff = Utc::now().timestamp() - LIVE_TTL_SECS;
        let mut stale_ids = Vec::new();
        self.mem_live.retain(|uid, val| {
            let keep = val.last_seen_epoch >= cutoff;
            if !keep {
                stale_ids.push(*uid);
            }
            keep
        });
        stale_ids
    }

    pub async fn live_snapshot_map(&self) -> HashMap<Uuid, LiveAccountState> {
        let cutoff = Utc::now().timestamp() - LIVE_TTL_SECS;
        let mut map: HashMap<Uuid, LiveAccountState> = self
            .mem_live
            .iter()
            .filter(|entry| entry.value().last_seen_epoch >= cutoff)
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect();

        let Some(pool) = &self.redis else {
            return map;
        };

        let cleanup_fut = pool.zremrangebyscore::<i64, _, _, _>(
            LIVE_ZSET_KEY,
            "-inf",
            cutoff as f64,
        );
        let _ = tokio::time::timeout(Duration::from_millis(REDIS_TIMEOUT_MS), cleanup_fut).await;

        let ids_res = tokio::time::timeout(
            Duration::from_millis(REDIS_TIMEOUT_MS),
            pool.zrevrange::<Vec<String>, _>(LIVE_ZSET_KEY, 0, 299, false),
        )
        .await;

        let Ok(Ok(user_strs)) = ids_res else {
            return map;
        };
        if user_strs.is_empty() {
            return map;
        }

        let keys: Vec<String> = user_strs
            .iter()
            .map(|id| format!("{LIVE_STATE_PREFIX}{id}"))
            .collect();
        let values_res = tokio::time::timeout(
            Duration::from_millis(REDIS_TIMEOUT_MS),
            pool.mget::<Vec<Option<String>>, _>(keys),
        )
        .await;

        if let Ok(Ok(values)) = values_res {
            for raw in values.into_iter().flatten() {
                if let Ok(parsed) = serde_json::from_str::<LiveAccountState>(&raw) {
                    if parsed.last_seen_epoch >= cutoff {
                        map.insert(parsed.user_id, parsed);
                    }
                }
            }
        }
        map
    }

    fn prune_if_needed(&self) {
        if self.mem_live.len() < MAX_MEM_LIVE_ENTRIES {
            return;
        }
        let expired = self.sweep_expired();
        if expired.is_empty() && self.mem_live.len() >= MAX_MEM_LIVE_ENTRIES {
            let mut oldest: Option<(Uuid, i64)> = None;
            for entry in self.mem_live.iter() {
                let ts = entry.value().last_seen_epoch;
                if oldest.is_none_or(|(_, min_ts)| ts < min_ts) {
                    oldest = Some((*entry.key(), ts));
                }
            }
            if let Some((uid, _)) = oldest {
                self.mem_live.remove(&uid);
            }
        }
    }
}
