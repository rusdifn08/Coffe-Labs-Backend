use std::{collections::VecDeque, sync::Arc, time::Duration};

use dashmap::DashMap;
use fred::{
    clients::RedisPool,
    interfaces::{KeysInterface, ListInterface, PubsubInterface},
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::modules::live_brew::{
    dto::{CatchupCurvePoint, WebRtcSignalPayload},
    protocol::ViewerDownstreamEvent,
};

const MAX_RAM_CURVE_POINTS: usize = 900;
const CHANNEL_CAP: usize = 128;

pub struct LiveTelemetryHub {
    pub redis_pool: Option<RedisPool>,
    session_channels: DashMap<Uuid, broadcast::Sender<Arc<ViewerDownstreamEvent>>>,
    table_waiting_channels: DashMap<String, broadcast::Sender<Arc<ViewerDownstreamEvent>>>,
    barista_cheer_channels: DashMap<Uuid, broadcast::Sender<String>>,
    ram_curve_buffers: DashMap<Uuid, VecDeque<CatchupCurvePoint>>,
    last_sampled_ms: DashMap<Uuid, u32>,
    whip_offers: DashMap<Uuid, String>,
    ice_candidates: DashMap<Uuid, Vec<serde_json::Value>>,
    cheer_guard: DashMap<String, std::time::Instant>,
}

impl LiveTelemetryHub {
    pub fn new(redis_pool: Option<RedisPool>) -> Self {
        Self {
            redis_pool,
            session_channels: DashMap::new(),
            table_waiting_channels: DashMap::new(),
            barista_cheer_channels: DashMap::new(),
            ram_curve_buffers: DashMap::new(),
            last_sampled_ms: DashMap::new(),
            whip_offers: DashMap::new(),
            ice_candidates: DashMap::new(),
            cheer_guard: DashMap::new(),
        }
    }

    pub fn subscribe_session(&self, sid: Uuid) -> broadcast::Receiver<Arc<ViewerDownstreamEvent>> {
        self.session_channels.entry(sid).or_insert_with(|| broadcast::channel(CHANNEL_CAP).0).subscribe()
    }

    pub fn subscribe_table_waiting(&self, code: &str) -> broadcast::Receiver<Arc<ViewerDownstreamEvent>> {
        self.table_waiting_channels
            .entry(code.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAP).0)
            .subscribe()
    }

    pub fn subscribe_barista_cheers(&self, sid: Uuid) -> broadcast::Receiver<String> {
        self.barista_cheer_channels.entry(sid).or_insert_with(|| broadcast::channel(64).0).subscribe()
    }

    pub fn broadcast_to_session(&self, session_id: Uuid, event: Arc<ViewerDownstreamEvent>) {
        if let Some(tx) = self.session_channels.get(&session_id) {
            let _ = tx.send(Arc::clone(&event));
        }
        if let Some(pool) = self.redis_pool.clone() {
            tokio::spawn(async move {
                if let Ok(json) = serde_json::to_string(event.as_ref()) {
                    let ch = format!("livebrew:stream:{session_id}");
                    let _ = tokio::time::timeout(Duration::from_millis(80), pool.next().publish::<i64, _, _>(ch, json)).await;
                }
            });
        }
    }

    pub fn broadcast_table_bound(&self, static_code: &str, event: Arc<ViewerDownstreamEvent>) {
        if let Some(tx) = self.table_waiting_channels.get(static_code) {
            let _ = tx.send(event);
        }
    }

    pub fn notify_barista_cheer(&self, session_id: Uuid, cheer_json: String) {
        if let Some(tx) = self.barista_cheer_channels.get(&session_id) {
            let _ = tx.send(cheer_json);
        }
    }

    pub fn check_cheer_rate_limit(&self, key: &str) -> bool {
        let now = std::time::Instant::now();
        let mut ok = true;
        self.cheer_guard
            .entry(key.to_string())
            .and_modify(|prev| {
                if now.duration_since(*prev) < Duration::from_secs(2) {
                    ok = false;
                } else {
                    *prev = now;
                }
            })
            .or_insert(now);
        if self.cheer_guard.len() > 2000 {
            self.cheer_guard.retain(|_, ts| now.duration_since(*ts) < Duration::from_secs(30));
        }
        ok
    }

    pub fn append_curve_point(&self, session_id: Uuid, pt: CatchupCurvePoint) {
        let keep = {
            let mut last = self.last_sampled_ms.entry(session_id).or_insert(0);
            if pt.t_ms == 0 || pt.t_ms.saturating_sub(*last) >= 450 {
                *last = pt.t_ms;
                true
            } else {
                false
            }
        };
        if !keep {
            return;
        }
        let mut ring = self.ram_curve_buffers.entry(session_id).or_default();
        if ring.len() >= MAX_RAM_CURVE_POINTS {
            ring.pop_front();
        }
        ring.push_back(pt.clone());

        if let Some(pool) = self.redis_pool.clone() {
            tokio::spawn(async move {
                if let Ok(encoded) = serde_json::to_string(&pt) {
                    let key = format!("livebrew:curve:{session_id}");
                    let _ = tokio::time::timeout(Duration::from_millis(80), async {
                        let _: Result<i64, _> = pool.rpush(&key, encoded).await;
                        let _: Result<bool, _> = pool.expire(&key, 7200).await;
                    })
                    .await;
                }
            });
        }
    }

    pub async fn get_catchup_curve(&self, session_id: Uuid) -> Vec<CatchupCurvePoint> {
        if let Some(ring) = self.ram_curve_buffers.get(&session_id) {
            if !ring.is_empty() {
                return ring.iter().cloned().collect();
            }
        }
        if let Some(pool) = &self.redis_pool {
            let key = format!("livebrew:curve:{session_id}");
            if let Ok(Ok(raw)) = tokio::time::timeout(Duration::from_millis(90), pool.lrange::<Vec<String>, _>(&key, 0, -1)).await {
                let pts: Vec<CatchupCurvePoint> = raw.into_iter().filter_map(|s| serde_json::from_str(&s).ok()).collect();
                if !pts.is_empty() {
                    return pts;
                }
            }
        }
        Vec::new()
    }

    pub fn store_whip_offer(&self, session_id: Uuid, sdp: String) {
        self.whip_offers.insert(session_id, sdp);
    }

    pub fn get_whip_offer(&self, session_id: Uuid) -> Option<String> {
        self.whip_offers.get(&session_id).map(|v| v.clone())
    }

    pub fn clear_whip(&self, session_id: Uuid) {
        self.whip_offers.remove(&session_id);
        self.ice_candidates.remove(&session_id);
    }

    pub fn push_ice_candidate(&self, session_id: Uuid, sig: &WebRtcSignalPayload) {
        let mut entry = self.ice_candidates.entry(session_id).or_default();
        if entry.len() < 32 {
            entry.push(serde_json::json!({
                "candidate": sig.candidate,
                "sdpMid": sig.sdp_mid,
                "sdpMLineIndex": sig.sdp_mline_index,
            }));
        }
    }

    pub fn take_ice_candidates(&self, session_id: Uuid) -> Vec<serde_json::Value> {
        self.ice_candidates.get(&session_id).map(|v| v.clone()).unwrap_or_default()
    }
}
