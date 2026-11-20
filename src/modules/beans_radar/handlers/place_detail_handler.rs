use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, OptionalAuthUser},
        AppState,
    },
    modules::beans_radar::{
        dto::{RadarPlaceCardDto, RadarPlaceDetailDto, ToggleFavoriteResponseDto},
        services::place_detail_service,
    },
};

#[derive(Debug, Clone, Deserialize)]
pub struct OriginCoordsQuery {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

pub async fn get_place_detail(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(place_id): Path<Uuid>,
    Query(coords): Query<OriginCoordsQuery>,
) -> AppResult<Json<ApiResponse<RadarPlaceDetailDto>>> {
    let viewer_id = maybe_claims.map(|c| c.sub);
    let data = place_detail_service::get_place_detail(
        &state,
        place_id,
        viewer_id,
        coords.lat,
        coords.lng,
    )
    .await?;
    Ok(ApiResponse::ok("Detail kedai kopi berhasil dimuat.", data))
}

pub async fn toggle_place_favorite(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(place_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<ToggleFavoriteResponseDto>>> {
    let data = place_detail_service::toggle_favorite(&state, claims.sub, place_id).await?;
    Ok(ApiResponse::ok("Status favorit kedai kopi diperbarui.", data))
}

pub async fn list_my_favorites(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(coords): Query<OriginCoordsQuery>,
) -> AppResult<Json<ApiResponse<Vec<RadarPlaceCardDto>>>> {
    let data =
        place_detail_service::list_favorites(&state, claims.sub, coords.lat, coords.lng).await?;
    Ok(ApiResponse::ok("Daftar kedai favorit Anda berhasil dimuat.", data))
}
