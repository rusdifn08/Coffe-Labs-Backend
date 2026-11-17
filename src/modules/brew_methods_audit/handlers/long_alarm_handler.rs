use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::AuthUser,
        AppState,
    },
    modules::brew_methods_audit::{
        dto::{ScheduleLongAlarmRequest, SnoozeAlarmRequest},
        repositories::long_alarm_repo,
        services::alarm_scheduler_service,
    },
};

pub async fn list_active_long_alarms(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let rows = long_alarm_repo::list_user_alarms(&state.pool, claims.sub)
        .await
        .map_err(AppError::from)?;
    let dtos: Vec<_> = rows
        .iter()
        .map(alarm_scheduler_service::alarm_row_to_dto)
        .collect();
    Ok(ApiResponse::ok("Daftar alarm Cold Brew aktif dimuat.", dtos))
}

pub async fn schedule_long_alarm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ScheduleLongAlarmRequest>,
) -> AppResult<impl IntoResponse> {
    let rpool = state.redis.active_pool();
    let dto = alarm_scheduler_service::schedule_alarm(&state.pool, rpool.as_ref(), claims.sub, &req)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::created("Alarm latar belakang Cold Brew berhasil dijadwalkan.", dto))
}

pub async fn snooze_long_alarm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(alarm_id): Path<Uuid>,
    Json(req): Json<SnoozeAlarmRequest>,
) -> AppResult<impl IntoResponse> {
    let rpool = state.redis.active_pool();
    let mins = req.snooze_minutes.unwrap_or(15);
    let dto = alarm_scheduler_service::snooze_alarm(
        &state.pool,
        rpool.as_ref(),
        claims.sub,
        alarm_id,
        mins,
    )
    .await
    .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Alarm Cold Brew ditunda sementara.", dto))
}

pub async fn complete_long_alarm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(alarm_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let rpool = state.redis.active_pool();
    let dto = alarm_scheduler_service::finish_or_cancel_alarm(
        &state.pool,
        rpool.as_ref(),
        claims.sub,
        alarm_id,
        "COMPLETED",
    )
    .await
    .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Ekstraksi Cold Brew selesai disaring & alarm dimatikan.", dto))
}

pub async fn cancel_long_alarm(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(alarm_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let rpool = state.redis.active_pool();
    let dto = alarm_scheduler_service::finish_or_cancel_alarm(
        &state.pool,
        rpool.as_ref(),
        claims.sub,
        alarm_id,
        "CANCELLED",
    )
    .await
    .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Alarm Cold Brew dibatalkan.", dto))
}
