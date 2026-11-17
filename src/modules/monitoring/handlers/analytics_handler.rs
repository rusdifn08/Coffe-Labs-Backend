use axum::{extract::State, response::IntoResponse};

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        AppState,
    },
    modules::monitoring::{
        handlers::accounts_monitor_handler::MonitoringViewer,
        services::overview_service,
    },
};

pub async fn get_monitoring_overview(
    State(state): State<AppState>,
    _viewer: MonitoringViewer,
) -> AppResult<impl IntoResponse> {
    let overview = overview_service::get_cached_overview(&state)
        .await
        .map_err(AppError::from)?;

    Ok(ApiResponse::ok(
        "Overview analitik pengguna & feature adoption berhasil dimuat.",
        overview.as_ref().clone(),
    ))
}
