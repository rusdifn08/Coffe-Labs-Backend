use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    core::telemetry::{EndpointMetricSummary, RequestTrace},
    modules::telemetry::models::{DatabasePoolMetrics, DomainEntityCounts, ServiceModuleInfo},
};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DashboardTelemetrySnapshot {
    pub app_name: String,
    pub version: String,
    pub allocator: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: i64,
    pub total_requests: u64,
    pub in_flight_requests: u64,
    pub requests_per_second: f64,
    pub global_avg_latency_ms: f64,
    pub global_p95_latency_ms: f64,
    pub global_p99_latency_ms: f64,
    pub status_2xx: u64,
    pub status_3xx: u64,
    pub status_4xx: u64,
    pub status_5xx: u64,
    pub error_rate_percent: f64,
    pub database: DatabasePoolMetrics,
    pub domain_counts: DomainEntityCounts,
    pub services: Vec<ServiceModuleInfo>,
    pub endpoints: Vec<EndpointMetricSummary>,
    pub recent_traces: Vec<RequestTrace>,
}
