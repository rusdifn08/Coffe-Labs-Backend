pub mod dto;
pub mod error;
pub mod handlers;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;

use std::sync::{Arc, Mutex};

use fred::clients::RedisPool;
use sqlx::PgPool;
use tokio::sync::mpsc::Receiver;

use crate::modules::monitoring::{
    models::MonitoringL1Caches,
    repositories::bulk_ingest_repo::PendingRawEvent,
    services::{
        batch_flusher_worker::{spawn_monitoring_workers, MonitoringIngestQueue},
        bitmap_analytics_service::BitmapAnalyticsService,
        live_presence_tracker::LivePresenceTracker,
    },
};

pub struct MonitoringEngine {
    pub live_presence: Arc<LivePresenceTracker>,
    pub bitmaps: Arc<BitmapAnalyticsService>,
    pub ingest_queue: MonitoringIngestQueue,
    pub l1_caches: MonitoringL1Caches,
    _rx_keepalive: Option<Mutex<Receiver<Vec<PendingRawEvent>>>>,
}

impl MonitoringEngine {
    pub fn init(redis_pool: Option<RedisPool>) -> Arc<Self> {
        let (ingest_queue, rx, pending_counter) = MonitoringIngestQueue::new();
        let _ = pending_counter;
        Arc::new(Self {
            live_presence: Arc::new(LivePresenceTracker::new(redis_pool)),
            bitmaps: Arc::new(BitmapAnalyticsService::new()),
            ingest_queue,
            l1_caches: MonitoringL1Caches::new(),
            _rx_keepalive: Some(Mutex::new(rx)),
        })
    }

    pub fn init_and_spawn(pool: PgPool, redis_pool: Option<RedisPool>) -> Arc<Self> {
        let (ingest_queue, rx, pending_counter) = MonitoringIngestQueue::new();
        let live_presence = Arc::new(LivePresenceTracker::new(redis_pool));
        let bitmaps = Arc::new(BitmapAnalyticsService::new());
        let l1_caches = MonitoringL1Caches::new();

        spawn_monitoring_workers(
            pool,
            rx,
            pending_counter,
            bitmaps.clone(),
            live_presence.clone(),
            l1_caches.clone(),
        );

        Arc::new(Self {
            live_presence,
            bitmaps,
            ingest_queue,
            l1_caches,
            _rx_keepalive: None,
        })
    }
}
