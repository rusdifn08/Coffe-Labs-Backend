use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crate::core::AppState;
use crate::modules::vision_dataset::dto::SyncVisionClassRequest;
use crate::modules::vision_dataset::service;

pub async fn get_classes(State(state): State<AppState>) -> impl IntoResponse {
    match service::get_vision_classes(&state.pool).await {
        Ok(classes) => (StatusCode::OK, Json(classes)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn sync_class(
    State(state): State<AppState>,
    Json(payload): Json<SyncVisionClassRequest>,
) -> impl IntoResponse {
    match service::sync_vision_class(&state.pool, payload).await {
        Ok(resp) => (StatusCode::OK, Json(resp)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
