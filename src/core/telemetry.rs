use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{atomic::{AtomicU64, Ordering}, Arc, RwLock};
use utoipa::ToSchema;

const MAX_RECENT_SAMPLES: usize = 128;
const MAX_RECENT_TRACES: usize = 80;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RequestTrace {
    pub id: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EndpointMetricSummary {
    pub route_key: String,
    pub method: String,
    pub path: String,
    pub hits: u64,
    pub errors: u64,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub last_status: u16,
    pub last_hit_at: DateTime<Utc>,
}

struct EndpointEntry {
    method: String,
    path: String,
    hits: u64,
    errors: u64,
    total_micros: u64,
    min_micros: u64,
    max_micros: u64,
    samples_micros: VecDeque<u64>,
    last_status: u16,
    last_hit_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct TelemetryRegistry {
    started_at: DateTime<Utc>,
    total_requests: Arc<AtomicU64>,
    in_flight: Arc<AtomicU64>,
    status_2xx: Arc<AtomicU64>,
    status_3xx: Arc<AtomicU64>,
    status_4xx: Arc<AtomicU64>,
    status_5xx: Arc<AtomicU64>,
    endpoints: Arc<DashMap<String, EndpointEntry>>,
    recent_traces: Arc<RwLock<VecDeque<RequestTrace>>>,
}

impl Default for TelemetryRegistry { fn default() -> Self { Self::new() } }

impl TelemetryRegistry {
    pub fn new() -> Self {
        Self {
            started_at: Utc::now(),
            total_requests: Arc::new(AtomicU64::new(0)),
            in_flight: Arc::new(AtomicU64::new(0)),
            status_2xx: Arc::new(AtomicU64::new(0)),
            status_3xx: Arc::new(AtomicU64::new(0)),
            status_4xx: Arc::new(AtomicU64::new(0)),
            status_5xx: Arc::new(AtomicU64::new(0)),
            endpoints: Arc::new(DashMap::new()),
            recent_traces: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_RECENT_TRACES))),
        }
    }

    pub fn inc_in_flight(&self) { self.in_flight.fetch_add(1, Ordering::Relaxed); }
    pub fn dec_in_flight(&self) { self.in_flight.fetch_sub(1, Ordering::Relaxed); }

    pub fn record(&self, method: &str, path: &str, status: u16, duration_micros: u64, req_id: String) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        match status {
            200..=299 => { self.status_2xx.fetch_add(1, Ordering::Relaxed); }
            300..=399 => { self.status_3xx.fetch_add(1, Ordering::Relaxed); }
            400..=499 => { self.status_4xx.fetch_add(1, Ordering::Relaxed); }
            _ => { self.status_5xx.fetch_add(1, Ordering::Relaxed); }
        }
        let key = format!("{} {}", method, path);
        let now = Utc::now();
        self.endpoints.entry(key).and_modify(|e| {
            e.hits += 1;
            if status >= 400 { e.errors += 1; }
            e.total_micros = e.total_micros.saturating_add(duration_micros);
            e.min_micros = e.min_micros.min(duration_micros);
            e.max_micros = e.max_micros.max(duration_micros);
            if e.samples_micros.len() >= MAX_RECENT_SAMPLES { e.samples_micros.pop_front(); }
            e.samples_micros.push_back(duration_micros);
            e.last_status = status;
            e.last_hit_at = now;
        }).or_insert_with(|| {
            let mut samples = VecDeque::with_capacity(MAX_RECENT_SAMPLES);
            samples.push_back(duration_micros);
            EndpointEntry {
                method: method.to_string(), path: path.to_string(), hits: 1,
                errors: u64::from(status >= 400), total_micros: duration_micros,
                min_micros: duration_micros, max_micros: duration_micros,
                samples_micros: samples, last_status: status, last_hit_at: now,
            }
        });
        if let Ok(mut guard) = self.recent_traces.write() {
            if guard.len() >= MAX_RECENT_TRACES { guard.pop_back(); }
            guard.push_front(RequestTrace {
                id: req_id, method: method.to_string(), path: path.to_string(),
                status, duration_ms: round2(duration_micros as f64 / 1000.0), timestamp: now,
            });
        }
    }

    pub fn snapshot_endpoints(&self) -> Vec<EndpointMetricSummary> {
        let mut list: Vec<EndpointMetricSummary> = self.endpoints.iter().map(|item| {
            let e = item.value();
            let mut sorted: Vec<u64> = e.samples_micros.iter().copied().collect();
            sorted.sort_unstable();
            let avg = if e.hits > 0 { (e.total_micros as f64 / e.hits as f64) / 1000.0 } else { 0.0 };
            EndpointMetricSummary {
                route_key: item.key().clone(),
                method: e.method.clone(),
                path: e.path.clone(),
                hits: e.hits,
                errors: e.errors,
                avg_ms: round2(avg),
                min_ms: round2(e.min_micros as f64 / 1000.0),
                max_ms: round2(e.max_micros as f64 / 1000.0),
                p50_ms: round2(percentile(&sorted, 0.50) as f64 / 1000.0),
                p95_ms: round2(percentile(&sorted, 0.95) as f64 / 1000.0),
                p99_ms: round2(percentile(&sorted, 0.99) as f64 / 1000.0),
                last_status: e.last_status,
                last_hit_at: e.last_hit_at,
            }
        }).collect();
        list.sort_by(|a, b| b.hits.cmp(&a.hits));
        list
    }

    pub fn snapshot_counts(&self) -> (DateTime<Utc>, u64, u64, u64, u64, u64, u64) {
        (
            self.started_at,
            self.total_requests.load(Ordering::Relaxed),
            self.in_flight.load(Ordering::Relaxed),
            self.status_2xx.load(Ordering::Relaxed),
            self.status_3xx.load(Ordering::Relaxed),
            self.status_4xx.load(Ordering::Relaxed),
            self.status_5xx.load(Ordering::Relaxed),
        )
    }

    pub fn snapshot_traces(&self) -> Vec<RequestTrace> {
        self.recent_traces.read().map(|g| g.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.status_2xx.store(0, Ordering::Relaxed);
        self.status_3xx.store(0, Ordering::Relaxed);
        self.status_4xx.store(0, Ordering::Relaxed);
        self.status_5xx.store(0, Ordering::Relaxed);
        self.endpoints.clear();
        if let Ok(mut g) = self.recent_traces.write() { g.clear(); }
    }
}

fn percentile(sorted: &[u64], pct: f64) -> u64 {
    if sorted.is_empty() { return 0; }
    let idx = ((sorted.len() as f64 - 1.0) * pct).round() as usize;
    sorted.get(idx).copied().unwrap_or(0)
}

fn round2(val: f64) -> f64 { (val * 100.0).round() / 100.0 }
