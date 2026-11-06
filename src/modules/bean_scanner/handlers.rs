use axum::{extract::State, response::IntoResponse, Json};
use crate::core::{
    error::{ApiResponse, AppResult},
    extractors::ValidatedJson,
    AppState,
};
use crate::modules::bean_scanner::{
    dto::BeanScanRequestDto,
    services::resolver_service::BeanResolverService,
    transfer_learning_dto::{RegisterTrainedModelRequest, RegisterTrainedModelResponse},
    transfer_learning_repo,
};

pub async fn scan_and_resolve_bean(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<BeanScanRequestDto>,
) -> AppResult<impl IntoResponse> {
    let resolved = BeanResolverService::resolve(&state.pool, payload).await?;
    Ok(ApiResponse::ok("Analisis cerdas kemasan biji kopi berhasil diselesaikan.", resolved))
}

pub async fn register_trained_model(
    State(state): State<AppState>,
    Json(payload): Json<RegisterTrainedModelRequest>,
) -> AppResult<impl IntoResponse> {
    let dims = payload.weights_vector.len();
    let class_slug = payload.class_slug.clone();
    let _id = transfer_learning_repo::upsert_trained_model(&state.pool, &payload).await?;

    let resp = RegisterTrainedModelResponse {
        success: true,
        class_slug,
        message: "Model Transfer Learning bobot MobileNetV3 berhasil disinkronisasi ke Cloud.".to_string(),
        dimensions: dims,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(ApiResponse::created("Transfer Learning model berhasil disimpan.", resp))
}

pub async fn get_trained_models(
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let list = transfer_learning_repo::list_trained_models(&state.pool).await?;
    Ok(ApiResponse::ok("Daftar model transfer learning kopi berhasil dimuat.", list))
}
