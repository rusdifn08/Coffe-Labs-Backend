use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::modules::monitoring::{
    models::MonitoringL1Caches,
    repositories::{
        bitmap_repo,
        bulk_ingest_repo::{flush_telemetry_batch, PendingRawEvent},
        user_sync_repo,
    },
    services::{
        bitmap_analytics_service::BitmapAnalyticsService,
        live_presence_tracker::LivePresenceTracker,
    },
};

pub const MPSC_QUEUE_CAPACITY: usize = 512;
const FLUSH_MAX_BATCH_EVENTS: usize = 200;
const FLUSH_INTERVAL_MS: u64 = 500;
const BITMAP_SAVE_INTERVAL_SECS: u64 = 15;
const PRESENCE_SWEEP_INTERVAL_SECS: u64 = 30;

/// Saluran MPSC berkapasitas terbatas (512 batch) untuk menyerap burst telemetri tanpa membebani 512MB RAM.
pub struct MonitoringIngestQueue {
    tx: mpsc::Sender<Vec<PendingRawEvent>>,
    pending_batches: Arc<AtomicUsize>,
    dropped_batches: Arc<AtomicU64>,
}

impl MonitoringIngestQueue {
    pub fn new() -> (Self, mpsc::Receiver<Vec<PendingRawEvent>>, Arc<AtomicUsize>) {
        let (tx, rx) = mpsc::channel(MPSC_QUEUE_CAPACITY);
        let pending_batches = Arc::new(AtomicUsize::new(0));
        let queue = Self {
            tx,
            pending_batches: pending_batches.clone(),
            dropped_batches: Arc::new(AtomicU64::new(0)),
        };
        (queue, rx, pending_batches)
    }

    pub fn enqueue(&self, batch: Vec<PendingRawEvent>) -> bool {
        if batch.is_empty() {
            return true;
        }
        match self.tx.try_send(batch) {
            Ok(()) => {
                self.pending_batches.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(_) => {
                self.dropped_batches.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending_batches.load(Ordering::Relaxed)
    }

    pub fn dropped_total(&self) -> u64 {
        self.dropped_batches.load(Ordering::Relaxed)
    }
}

pub fn spawn_monitoring_workers(
    pool: PgPool,
    mut rx: mpsc::Receiver<Vec<PendingRawEvent>>,
    pending_counter: Arc<AtomicUsize>,
    bitmaps: Arc<BitmapAnalyticsService>,
    live_presence: Arc<LivePresenceTracker>,
    l1_caches: MonitoringL1Caches,
) {
    tokio::spawn(async move {
        user_sync_repo::sync_existing_users_to_monitoring(&pool).await;
        bitmap_repo::load_recent_bitmaps_from_db(&pool, &bitmaps).await;

        let mut buffer: Vec<PendingRawEvent> = Vec::with_capacity(FLUSH_MAX_BATCH_EVENTS);
        let mut flush_tick = tokio::time::interval(Duration::from_millis(FLUSH_INTERVAL_MS));
        let mut bitmap_tick = tokio::time::interval(Duration::from_secs(BITMAP_SAVE_INTERVAL_SECS));
        let mut sweep_tick = tokio::time::interval(Duration::from_secs(PRESENCE_SWEEP_INTERVAL_SECS));

        info!(
            queue_cap = MPSC_QUEUE_CAPACITY,
            flush_ms = FLUSH_INTERVAL_MS,
            "⚙️ MonitoringBatchFlusher & RoaringBitmap worker aktif (512MB RAM bounded)"
        );

        loop {
            tokio::select! {
                maybe_batch = rx.recv() => {
                    match maybe_batch {
                        Some(batch) => {
                            pending_counter.fetch_sub(1, Ordering::Relaxed);
                            buffer.extend(batch);
                            if buffer.len() >= FLUSH_MAX_BATCH_EVENTS {
                                flush_and_record(&pool, &mut buffer, &bitmaps, &l1_caches).await;
                            }
                        }
                        None => {
                            flush_and_record(&pool, &mut buffer, &bitmaps, &l1_caches).await;
                            let dirty = bitmaps.drain_dirty_snapshots();
                            bitmap_repo::save_bitmap_snapshots(&pool, &dirty).await;
                            break;
                        }
                    }
                }
                _ = flush_tick.tick() => {
                    if !buffer.is_empty() {
                        flush_and_record(&pool, &mut buffer, &bitmaps, &l1_caches).await;
                    }
                }
                _ = bitmap_tick.tick() => {
                    let today = Utc::now().date_naive();
                    bitmaps.evict_old_days(today);
                    let dirty = bitmaps.drain_dirty_snapshots();
                    bitmap_repo::save_bitmap_snapshots(&pool, &dirty).await;
                }
                _ = sweep_tick.tick() => {
                    let stale_ids = live_presence.sweep_expired();
                    if !stale_ids.is_empty() {
                        user_sync_repo::mark_users_offline(&pool, &stale_ids).await;
                        l1_caches.invalidate_reads();
                    }
                }
            }
        }
    });
}

async fn flush_and_record(
    pool: &PgPool,
    buffer: &mut Vec<PendingRawEvent>,
    bitmaps: &BitmapAnalyticsService,
    l1_caches: &MonitoringL1Caches,
) {
    if buffer.is_empty() {
        return;
    }
    let batch = std::mem::take(buffer);
    let mut user_ids: Vec<uuid::Uuid> = batch.iter().map(|e| e.user_id).collect();
    user_ids.sort_unstable();
    user_ids.dedup();

    let seq_map = user_sync_repo::lookup_bitmap_seq_and_tiers(pool, &user_ids).await;
    for ev in &batch {
        if let Some((seq_id, tier)) = seq_map.get(&ev.user_id) {
            bitmaps.record_user_activity(
                ev.recorded_at.date_naive(),
                *seq_id,
                &ev.feature_bucket,
                tier,
                ev.aod_kiosk_sec > 0 || ev.feature_bucket == "aod_kiosk",
            );
        }
    }

    if let Err(err) = flush_telemetry_batch(pool, &batch).await {
        warn!(error = %err, count = batch.len(), "Gagal flush batch telemetri ke NeonDB");
    } else {
        l1_caches.invalidate_reads();
    }
}
