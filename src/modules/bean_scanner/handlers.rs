use axum::{extract::State, response::IntoResponse};
use crate::core::{
    error::{ApiResponse, AppResult},
    extractors::ValidatedJson,
    AppState,
};
use crate::modules::bean_scanner::{
    dto::BeanScanRequestDto,
    services::resolver_service::BeanResolverService,
};

pub async fn scan_and_resolve_bean(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<BeanScanRequestDto>,
) -> AppResult<impl IntoResponse> {
    let resolved = BeanResolverService::resolve(&state.pool, payload).await?;
    Ok(ApiResponse::ok("Analisis cerdas kemasan biji kopi berhasil diselesaikan.", resolved))
}
