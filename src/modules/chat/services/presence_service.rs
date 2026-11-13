use chrono::{DateTime, Duration as ChronoDuration, Utc};
use dashmap::DashMap;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::chat::{
    protocol::ServerFrame,
    repositories::peer_repo,
    services::redis_pubsub_bridge::RedisPubSubBridge,
};

const PRESENCE_LEASE_SECONDS: i64 = 45;

#[derive(Debug, Clone)]
struct PresenceEntry {
    online_sessions: u32,
    last_seen_at: Option<DateTime<Utc>>,
    last_ping_at: DateTime<Utc>,
}

/// Ephemeral RAM/Redis online & last-seen registry (zero PostgreSQL writes on heartbeats).
pub struct PresenceService {
    entries: DashMap<Uuid, PresenceEntry>,
    active_rooms: DashMap<Uuid, Uuid>,
}

impl Default for PresenceService {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceService {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            active_rooms: DashMap::new(),
        }
    }

    pub fn set_active_room(&self, user_id: Uuid, conversation_id: Option<Uuid>) {
        match conversation_id {
            Some(room_id) => {
                self.active_rooms.insert(user_id, room_id);
            }
            None => {
                self.active_rooms.remove(&user_id);
            }
        }
    }

    pub fn active_room(&self, user_id: Uuid) -> Option<Uuid> {
        self.active_rooms.get(&user_id).map(|v| *v.value())
    }

    pub fn mark_online(&self, user_id: Uuid, bridge: &RedisPubSubBridge) {
        let now = Utc::now();
        self.entries
            .entry(user_id)
            .and_modify(|e| {
                e.online_sessions = e.online_sessions.saturating_add(1);
                e.last_ping_at = now;
            })
            .or_insert(PresenceEntry {
                online_sessions: 1,
                last_seen_at: None,
                last_ping_at: now,
            });
        bridge.refresh_presence_lease(user_id, PRESENCE_LEASE_SECONDS);
    }

    pub fn refresh_heartbeat(&self, user_id: Uuid, bridge: &RedisPubSubBridge) {
        let now = Utc::now();
        if let Some(mut entry) = self.entries.get_mut(&user_id) {
            entry.last_ping_at = now;
        }
        bridge.refresh_presence_lease(user_id, PRESENCE_LEASE_SECONDS);
    }

    pub fn mark_offline(&self, user_id: Uuid, bridge: &RedisPubSubBridge) -> DateTime<Utc> {
        let now = Utc::now();
        let mut no_more_sessions = false;
        self.entries
            .entry(user_id)
            .and_modify(|e| {
                e.online_sessions = e.online_sessions.saturating_sub(1);
                if e.online_sessions == 0 {
                    e.last_seen_at = Some(now);
                    no_more_sessions = true;
                }
            })
            .or_insert(PresenceEntry {
                online_sessions: 0,
                last_seen_at: Some(now),
                last_ping_at: now,
            });

        if no_more_sessions {
            self.active_rooms.remove(&user_id);
            bridge.clear_presence_lease(user_id);
        }
        now
    }

    pub fn snapshot(&self, user_id: Uuid) -> (bool, Option<DateTime<Utc>>) {
        let now = Utc::now();
        match self.entries.get(&user_id) {
            Some(entry) => {
                let lease_alive = now - entry.last_ping_at
                    <= ChronoDuration::seconds(PRESENCE_LEASE_SECONDS + 10);
                let is_online = entry.online_sessions > 0 && lease_alive;
                let last_seen = if is_online {
                    None
                } else {
                    entry.last_seen_at.or(Some(entry.last_ping_at))
                };
                (is_online, last_seen)
            }
            None => (false, None),
        }
    }
}

pub async fn broadcast_presence_change(
    pool: &PgPool,
    bridge: &RedisPubSubBridge,
    user_id: Uuid,
    is_online: bool,
    last_seen_at: Option<DateTime<Utc>>,
) {
    let Ok(peer_ids) = peer_repo::get_peer_contact_ids(pool, user_id).await else {
        return;
    };
    if peer_ids.is_empty() {
        return;
    }

    let frame = Arc::new(ServerFrame::PresenceUpdate {
        user_id,
        is_online,
        last_seen_at,
    });
    bridge.publish_to_users(&peer_ids, frame).await;
}
