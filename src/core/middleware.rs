use axum::{
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor, GovernorLayer,
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::core::{error::AppError, telemetry::TelemetryRegistry};

#[derive(Clone)]
pub struct RateLimitBucket {
    pub count: u32,
    pub window_start: Instant,
}

#[derive(Clone)]
pub struct SecurityRateLimiter {
    buckets: Arc<DashMap<String, RateLimitBucket>>,
    max_per_window: u32,
    window: Duration,
}

impl SecurityRateLimiter {
    pub fn new(max_per_sec: u64, burst: u32) -> Self {
        let max_per_window = ((max_per_sec as u32).saturating_mul(20)).max(burst).max(2000);
        Self {
            buckets: Arc::new(DashMap::new()),
            max_per_window,
            window: Duration::from_secs(10),
        }
    }

    pub fn check_key(&self, key: &str) -> Result<(), AppError> {
        let now = Instant::now();
        let mut allowed = true;

        self.buckets
            .entry(key.to_string())
            .and_modify(|b| {
                if now.duration_since(b.window_start) >= self.window {
                    b.count = 1;
                    b.window_start = now;
                } else {
                    b.count += 1;
                    if b.count > self.max_per_window {
                        allowed = false;
                    }
                }
            })
            .or_insert(RateLimitBucket {
                count: 1,
                window_start: now,
            });

        if allowed {
            Ok(())
        } else {
            Err(AppError::RateLimited(
                "Terlalu banyak permintaan dalam waktu singkat. Silakan tunggu beberapa detik.".to_string(),
            ))
        }
    }
}

pub fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

pub fn attach_governor_layer<S>(
    router: axum::Router<S>,
    per_second: u64,
    burst_size: u32,
) -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let replenish_ms = (1000 / per_second.max(1)).clamp(1, 10);
    let effective_burst = burst_size.max(5000);
    if let Some(conf) = GovernorConfigBuilder::default()
        .per_millisecond(replenish_ms)
        .burst_size(effective_burst)
        .key_extractor(GlobalKeyExtractor)
        .finish()
    {
        router.layer(GovernorLayer {
            config: Arc::new(conf),
        })
    } else {
        router
    }
}

pub async fn telemetry_and_security_middleware(
    State((telemetry, limiter)): State<(TelemetryRegistry, SecurityRateLimiter)>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let start = Instant::now();
    let method = req.method().as_str().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    let client_key = req
        .headers()
        .get("x-forwarded-for")
        .or_else(|| req.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("global_client")
        .to_string();

    let is_exempt = path == "/"
        || path == "/health"
        || path == "/api/health"
        || path.starts_with("/dashboard")
        || path.starts_with("/api/v1/telemetry")
        || path.starts_with("/api/v1/app/version-check");

    if !is_exempt {
        limiter.check_key(&client_key)?;
    }

    let req_id = Uuid::new_v4().to_string();
    telemetry.inc_in_flight();
    let mut response = next.run(req).await;
    telemetry.dec_in_flight();

    let elapsed_micros = start.elapsed().as_micros() as u64;
    let status = response.status().as_u16();
    telemetry.record(&method, &path, status, elapsed_micros, req_id.clone());

    let headers = response.headers_mut();
    if let Ok(val) = HeaderValue::from_str(&req_id) {
        headers.insert(HeaderName::from_static("x-request-id"), val);
    }
    if let Ok(val) = HeaderValue::from_str(&format!("{:.2}ms", elapsed_micros as f64 / 1000.0)) {
        headers.insert(HeaderName::from_static("x-response-time"), val);
    }
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("SAMEORIGIN"),
    );

    Ok(response)
}
