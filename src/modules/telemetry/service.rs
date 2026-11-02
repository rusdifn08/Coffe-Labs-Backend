use chrono::Utc;
use sqlx::PgPool;

use crate::{
    core::telemetry::TelemetryRegistry,
    modules::telemetry::{
        dto::DashboardTelemetrySnapshot, models::ServiceModuleInfo, repository,
    },
};

pub fn get_service_catalog() -> Vec<ServiceModuleInfo> {
    let entries = [
        ("auth", "Hardened Auth & Session Engine", "/api/v1/auth", "Login, Register, Ed25519 JWT, Token Family Rotation", "Zero-Trust + Argon2id"),
        ("rbac", "Tiered RBAC & Quota Gatekeeper", "/api/v1/rbac", "Trial (10 Brews Lock), Pro (Rp 99k), Business (Rp 249k)", "RequireTier<T> Guard"),
        ("social", "Social Coffee Community Engine", "/api/v1/social", "Infinite Scroll Cursor Feed, Recipe Shares, Nested Comments, Follows", "OptionalAuth + Validator"),
        ("users", "Barista Profile & Admin Management", "/api/admin/users", "User Approval, Equipment JSON, Barista Stats", "AdminUser + AuthUser"),
        ("recipes", "Cloud Recipe Sync & Favorites", "/api/recipes", "Custom Recipes Sync, WBC/WBrC Catalog, Favorites", "Hybrid Offline-Cloud"),
        ("brew_logs", "Brew History, Cupping & Smart Scale", "/api/brew-logs", "Flow Curve Telemetry, Sensory Evaluation Journal, Analytics", "Auth + Guest Sync"),
        ("beans", "Curated Beans, AI Dial-In & Radar", "/api/beans", "Roastery Bean Catalog, AI Barista Pour Schedule, Bean Radar", "ValidatedJson DTO"),
        ("marketplace", "Roastery Marketplace & Vouchers", "/api/v1/marketplace", "Specialty Coffee Cart, Vouchers (COFFEEFIRST/BARISTA20), Thermal Orders", "Zero-Trust Checkout"),
        ("platform", "Hybrid 3-Tier Version Gatekeeper", "/api/v1/app/version-check", "Shorebird OTA Patch, Store Update & User Feedback System", "RwLock In-Memory Cache"),
        ("telemetry", "Real-Time APM & Latency Dashboard", "/dashboard", "Per-API P50/P95/P99 Response Time, RPS, Trace Stream", "Lock-Free DashMap"),
    ];

    entries
        .into_iter()
        .map(|(module, name, base_path, mobile_feature, security_tier)| ServiceModuleInfo {
            module: module.to_string(),
            name: name.to_string(),
            base_path: base_path.to_string(),
            mobile_feature: mobile_feature.to_string(),
            security_tier: security_tier.to_string(),
            status: "HEALTHY".to_string(),
        })
        .collect()
}

pub async fn build_snapshot(
    pool: &PgPool,
    telemetry: &TelemetryRegistry,
) -> DashboardTelemetrySnapshot {
    let (started_at, total, in_flight, s2xx, s3xx, s4xx, s5xx) = telemetry.snapshot_counts();
    let endpoints = telemetry.snapshot_endpoints();
    let recent_traces = telemetry.snapshot_traces();
    let (database, domain_counts) = repository::probe_database(pool).await;

    let uptime_seconds = (Utc::now() - started_at).num_seconds().max(1);
    let rps = ((total as f64 / uptime_seconds as f64) * 100.0).round() / 100.0;

    let (global_avg, global_p95, global_p99) = if endpoints.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        let total_hits: u64 = endpoints.iter().map(|e| e.hits).sum();
        let weighted_avg = if total_hits > 0 {
            endpoints.iter().map(|e| e.avg_ms * e.hits as f64).sum::<f64>() / total_hits as f64
        } else {
            0.0
        };
        let max_p95 = endpoints.iter().map(|e| e.p95_ms).fold(0.0_f64, f64::max);
        let max_p99 = endpoints.iter().map(|e| e.p99_ms).fold(0.0_f64, f64::max);
        (
            (weighted_avg * 100.0).round() / 100.0,
            (max_p95 * 100.0).round() / 100.0,
            (max_p99 * 100.0).round() / 100.0,
        )
    };

    let err_rate = if total > 0 {
        (((s4xx + s5xx) as f64 / total as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };

    DashboardTelemetrySnapshot {
        app_name: "Coffee Labs Enterprise Rust Backend".to_string(),
        version: "2.0.0".to_string(),
        allocator: "mimalloc (Global Lock-Free Allocator)".to_string(),
        started_at,
        uptime_seconds,
        total_requests: total,
        in_flight_requests: in_flight,
        requests_per_second: rps,
        global_avg_latency_ms: global_avg,
        global_p95_latency_ms: global_p95,
        global_p99_latency_ms: global_p99,
        status_2xx: s2xx,
        status_3xx: s3xx,
        status_4xx: s4xx,
        status_5xx: s5xx,
        error_rate_percent: err_rate,
        database,
        domain_counts,
        services: get_service_catalog(),
        endpoints,
        recent_traces,
    }
}
