use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::core::{
    error::{ApiResponse, AppResult},
    AppState,
};
use crate::modules::roasteries::{
    dto::PublicRoasteryQuery,
    models::RoasteryWithLineupsResponse,
    services::roastery_catalog_service,
};

pub async fn list_public_roasteries(
    State(state): State<AppState>,
    Query(query): Query<PublicRoasteryQuery>,
) -> AppResult<Json<ApiResponse<Vec<RoasteryWithLineupsResponse>>>> {
    let list = roastery_catalog_service::list_public_roasteries(&state, &query).await?;
    Ok(ApiResponse::ok("Katalog Roastery Bisnis berhasil dimuat.", list))
}

pub async fn get_public_roastery_detail(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> AppResult<Json<ApiResponse<RoasteryWithLineupsResponse>>> {
    let detail = roastery_catalog_service::get_public_roastery_detail(&state, &id_or_slug).await?;
    Ok(ApiResponse::ok("Detail Roastery berhasil dimuat.", detail))
}
