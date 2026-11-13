use dashmap::DashMap;
use smallvec::SmallVec;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;

use crate::modules::chat::protocol::ServerFrame;

const SOCKET_CHANNEL_CAPACITY: usize = 128;

#[derive(Clone)]
pub struct DeviceSession {
    pub session_id: Uuid,
    pub tx: mpsc::Sender<Arc<ServerFrame>>,
}

/// Lock-free per-node connection registry supporting up to 4 concurrent devices per user on stack.
pub struct ConnectionHub {
    connections: DashMap<Uuid, SmallVec<[DeviceSession; 4]>>,
}

impl Default for ConnectionHub {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionHub {
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    pub fn register(
        &self,
        user_id: Uuid,
    ) -> (Uuid, mpsc::Receiver<Arc<ServerFrame>>, bool) {
        let session_id = Uuid::new_v4();
        let (tx, rx) = mpsc::channel(SOCKET_CHANNEL_CAPACITY);
        let session = DeviceSession { session_id, tx };

        let mut became_online = false;
        self.connections
            .entry(user_id)
            .and_modify(|list| {
                if list.is_empty() {
                    became_online = true;
                }
                list.push(session.clone());
            })
            .or_insert_with(|| {
                became_online = true;
                let mut vec = SmallVec::new();
                vec.push(session);
                vec
            });

        (session_id, rx, became_online)
    }

    pub fn unregister(&self, user_id: Uuid, session_id: Uuid) -> bool {
        let mut became_offline = false;
        let mut should_remove_entry = false;

        if let Some(mut entry) = self.connections.get_mut(&user_id) {
            entry.retain(|s| s.session_id != session_id);
            if entry.is_empty() {
                should_remove_entry = true;
                became_offline = true;
            }
        }

        if should_remove_entry {
            self.connections.remove_if(&user_id, |_, list| list.is_empty());
        }

        became_offline
    }

    /// Zero-copy local fan-out to all connected devices of `user_id`.
    pub fn send_local(&self, user_id: Uuid, frame: Arc<ServerFrame>) -> usize {
        let mut delivered_sessions = 0usize;
        if let Some(sessions) = self.connections.get(&user_id) {
            for session in sessions.iter() {
                match session.tx.try_send(Arc::clone(&frame)) {
                    Ok(()) => delivered_sessions += 1,
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        warn!(
                            user_id = %user_id,
                            session_id = %session.session_id,
                            "WebSocket outbound queue penuh; menjatuhkan frame lambat"
                        );
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {}
                }
            }
        }
        delivered_sessions
    }

    pub fn is_local_online(&self, user_id: Uuid) -> bool {
        self.connections
            .get(&user_id)
            .map(|list| !list.is_empty())
            .unwrap_or(false)
    }

    pub fn active_user_count(&self) -> usize {
        self.connections.len()
    }
}
