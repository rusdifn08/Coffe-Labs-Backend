use std::{
    sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    },
    time::Duration,
};

use chrono::Utc;
use dashmap::DashMap;
use fred::{
    clients::RedisPool,
    interfaces::{KeysInterface, PubsubInterface},
    types::Expiration,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::modules::live_brew::{
    mirror_dto::{BrandedSeatResolutionDto, LiveSessionMirrorSnapshot},
    mirror_protocol::MirrorDownstreamFrame,
};

const MIRROR_CHAN_CAP: usize = 128;
const REDIS_SESSION_TTL_SECS: i64 = 10_800;

/// Hub Zero-Copy Siaran Mirror 1:1 (DashMap + Redis Hash/PubSub) untuk sesi & meja/kursi.
pub struct MirrorStateHub {
    redis: Option<RedisPool>,
    session_channels: DashMap<Uuid, broadcast::Sender<Arc<MirrorDownstreamFrame>>>,
    seat_channels: DashMap<Uuid, broadcast::Sender<Arc<MirrorDownstreamFrame>>>,
    session_snapshots: DashMap<Uuid, LiveSessionMirrorSnapshot>,
    viewer_counts: DashMap<Uuid, AtomicI32>,
}

impl MirrorStateHub {
    pub fn new(redis: Option<RedisPool>) -> Self {
        Self {
            redis,
            session_channels: DashMap::new(),
            seat_channels: DashMap::new(),
            session_snapshots: DashMap::new(),
            viewer_counts: DashMap::new(),
        }
    }

    pub fn subscribe_session(&self, sid: Uuid) -> broadcast::Receiver<Arc<MirrorDownstreamFrame>> {
        self.session_channels
            .entry(sid)
            .or_insert_with(|| broadcast::channel(MIRROR_CHAN_CAP).0)
            .subscribe()
    }

    pub fn subscribe_seat(&self, seat_id: Uuid) -> broadcast::Receiver<Arc<MirrorDownstreamFrame>> {
        self.seat_channels
            .entry(seat_id)
            .or_insert_with(|| broadcast::channel(MIRROR_CHAN_CAP).0)
            .subscribe()
    }

    pub fn get_cached_snapshot(&self, sid: Uuid) -> Option<LiveSessionMirrorSnapshot> {
        self.session_snapshots.get(&sid).map(|r| {
            let mut s = r.clone();
            s.server_epoch_ms = Utc::now().timestamp_millis();
            s.active_viewers_count = self.active_viewers(sid);
            s
        })
    }

    pub fn active_viewers(&self, sid: Uuid) -> i32 {
        self.viewer_counts
            .get(&sid)
            .map(|c| c.load(Ordering::Relaxed).max(0))
            .unwrap_or(0)
    }

    pub fn adjust_viewers(&self, sid: Uuid, delta: i32) -> i32 {
        let counter = self.viewer_counts.entry(sid).or_insert_with(|| AtomicI32::new(0));
        let next = (counter.fetch_add(delta, Ordering::Relaxed) + delta).max(0);
        if next == 0 {
            counter.store(0, Ordering::Relaxed);
        }
        let ev = Arc::new(MirrorDownstreamFrame::ViewerCountUpdated { active_viewers: next });
        if let Some(tx) = self.session_channels.get(&sid) {
            let _ = tx.send(ev);
        }
        next
    }

    pub fn publish_snapshot(&self, snap: LiveSessionMirrorSnapshot) {
        let sid = snap.session_id;
        let seat_id = snap.seat_id;
        self.session_snapshots.insert(sid, snap.clone());
        let frame = Arc::new(MirrorDownstreamFrame::MirrorSnapshot(snap.clone()));
        if let Some(tx) = self.session_channels.get(&sid) {
            let _ = tx.send(Arc::clone(&frame));
        }
        if let Some(stid) = seat_id {
            if let Some(tx) = self.seat_channels.get(&stid) {
                let _ = tx.send(Arc::clone(&frame));
            }
        }
        if let Some(pool) = self.redis.clone() {
            tokio::spawn(async move {
                if let Ok(json) = serde_json::to_string(&snap) {
                    let key = format!("livebrew:session:{sid}");
                    let ch = format!("livebrew:mirror:{sid}");
                    let _ = tokio::time::timeout(Duration::from_millis(80), async {
                        let _: Result<(), _> = pool
                            .set(&key, json.clone(), Some(Expiration::EX(REDIS_SESSION_TTL_SECS)), None, false)
                            .await;
                        let _: Result<i64, _> = pool.next().publish(ch, json).await;
                    })
                    .await;
                }
            });
        }
    }

    pub fn publish_seat_update(&self, seat_id: Uuid, resolution: BrandedSeatResolutionDto) {
        let frame = Arc::new(MirrorDownstreamFrame::SeatPosUpdate(resolution));
        if let Some(tx) = self.seat_channels.get(&seat_id) {
            let _ = tx.send(frame);
        }
    }

    pub fn publish_cheer(&self, sid: Uuid, cheer_kind: String, total_cheers: i32) {
        let frame = Arc::new(MirrorDownstreamFrame::CheerBurst {
            cheer_kind,
            total_cheers,
        });
        if let Some(tx) = self.session_channels.get(&sid) {
            let _ = tx.send(frame);
        }
    }
}
