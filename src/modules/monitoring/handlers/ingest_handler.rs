use std::io::Read;

use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use chrono::Utc;
use flate2::read::GzDecoder;
use validator::Validate;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::monitoring::{
        dto::{BatchIngestResponse, HeartbeatRequest, TelemetryBatchRequest, MAX_BATCH_ITEMS},
        error::MonitoringError,
        models::LiveAccountState,
        repositories::bulk_ingest_repo::PendingRawEvent,
        services::dossier_aggregator::{normalize_feature_bucket, sanitize_event_duration},
    },
};

const MAX_DECOMPRESSED_BYTES: usize = 1_048_576;

fn decode_batch_body(headers: &HeaderMap, body: &Bytes) -> Result<TelemetryBatchRequest, MonitoringError> {
    let is_gzip = headers
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|enc| enc.to_ascii_lowercase().contains("gzip"));

    let raw_slice: Vec<u8> = if is_gzip {
        let mut decoder = GzDecoder::new(body.as_ref()).take((MAX_DECOMPRESSED_BYTES + 1) as u64);
        let mut out = Vec::with_capacity((body.len() * 3).min(MAX_DECOMPRESSED_BYTES));
        decoder
            .read_to_end(&mut out)
            .map_err(|_| MonitoringError::InvalidGzip)?;
        if out.len() > MAX_DECOMPRESSED_BYTES {
            return Err(MonitoringError::PayloadTooLarge);
        }
        out
    } else {
        if body.len() > MAX_DECOMPRESSED_BYTES {
            return Err(MonitoringError::PayloadTooLarge);
        }
        body.to_vec()
    };

    let req: TelemetryBatchRequest = serde_json::from_slice(&raw_slice)
        .map_err(|e| MonitoringError::InvalidPayload(e.to_string()))?;
    req.validate()
        .map_err(|e| MonitoringError::InvalidPayload(e.to_string()))?;
    Ok(req)
}

pub async fn ingest_batch(
    State(state): State<AppState>,
    OptionalAuthUser(opt_claims): OptionalAuthUser,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<(StatusCode, impl IntoResponse)> {
    let req = decode_batch_body(&headers, &body).map_err(AppError::from)?;
    let now = Utc::now();

    let Some(claims) = opt_claims else {
        return Ok((
            StatusCode::ACCEPTED,
            ApiResponse::ok(
                "Batch telemetri tamu diterima (mode anonim).",
                BatchIngestResponse {
                    accepted_events: 0,
                    dropped_backpressure: 0,
                    server_time: now,
                },
            ),
        ));
    };

    let mut events = Vec::with_capacity(req.events.len().min(MAX_BATCH_ITEMS));
    for ev in req.events.into_iter().take(MAX_BATCH_ITEMS) {
        let dur = sanitize_event_duration(
            &ev.event_type,
            &ev.feature_bucket,
            ev.duration_sec,
            ev.is_aod_mode,
        );
        let bucket = normalize_feature_bucket(&ev.feature_bucket).to_string();
        events.push(PendingRawEvent {
            event_id: ev.event_id,
            user_id: claims.sub,
            session_id: ev.session_id,
            event_type: ev.event_type,
            feature_bucket: bucket,
            route_name: ev.route_name,
            active_screen_sec: dur.active_screen_sec,
            aod_kiosk_sec: dur.aod_kiosk_sec,
            metadata_json: ev.metadata_json,
            recorded_at: ev.recorded_at,
            platform: req.platform.clone(),
            app_version: req.app_version.clone(),
            device_model: req.device_model.clone(),
        });
    }

    let last_ev = events.last();
    let live_route = req
        .active_route
        .clone()
        .or_else(|| last_ev.map(|e| e.route_name.clone()))
        .unwrap_or_else(|| "/".to_string());
    let live_feat = req
        .active_feature
        .as_deref()
        .map(normalize_feature_bucket)
        .map(str::to_string)
        .or_else(|| last_ev.map(|e| e.feature_bucket.clone()))
        .unwrap_or_else(|| "general".to_string());

    state
        .monitoring
        .live_presence
        .upsert_live_state(LiveAccountState {
            user_id: claims.sub,
            current_route: live_route,
            feature_bucket: live_feat,
            tier: claims.tier,
            platform: req.platform,
            app_version: req.app_version.unwrap_or_default(),
            is_brewing_now: req.is_brewing,
            last_seen_epoch: now.timestamp(),
        })
        .await;

    let count = events.len();
    let queued = state.monitoring.ingest_queue.enqueue(events);

    Ok((
        StatusCode::ACCEPTED,
        ApiResponse::ok(
            "Batch telemetri berhasil diantrekan.",
            BatchIngestResponse {
                accepted_events: if queued { count } else { 0 },
                dropped_backpressure: if queued { 0 } else { count },
                server_time: now,
            },
        ),
    ))
}

pub async fn heartbeat(
    State(state): State<AppState>,
    OptionalAuthUser(opt_claims): OptionalAuthUser,
    ValidatedJson(req): ValidatedJson<HeartbeatRequest>,
) -> AppResult<impl IntoResponse> {
    let now = Utc::now();
    if let Some(claims) = opt_claims {
        state
            .monitoring
            .live_presence
            .upsert_live_state(LiveAccountState {
                user_id: claims.sub,
                current_route: req.active_route,
                feature_bucket: normalize_feature_bucket(&req.feature_bucket).to_string(),
                tier: claims.tier,
                platform: req.platform,
                app_version: req.app_version.unwrap_or_default(),
                is_brewing_now: req.is_brewing,
                last_seen_epoch: now.timestamp(),
            })
            .await;
    }

    Ok(ApiResponse::ok(
        "Heartbeat kehadiran diperbarui.",
        serde_json::json!({ "server_time": now }),
    ))
}
