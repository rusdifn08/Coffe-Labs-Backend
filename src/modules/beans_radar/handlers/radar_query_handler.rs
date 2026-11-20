use axum::{
    extract::{Query, State},
    Json,
};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        AppState,
    },
    modules::beans_radar::{
        dto::{
            NearbyQueryParams, NearbyRadarResponseDto, RadarPlaceCardDto, RadarStatsDto,
            SearchQueryParams, ViewportQueryParams,
        },
        services::{place_detail_service, radar_discovery_service},
    },
};

pub async fn get_nearby_places(
    State(state): State<AppState>,
    Query(params): Query<NearbyQueryParams>,
) -> AppResult<Json<ApiResponse<NearbyRadarResponseDto>>> {
    let data = radar_discovery_service::query_nearby(&state, &params).await?;
    Ok(ApiResponse::ok(
        "Daftar kedai kopi & roastery terdekat berhasil dimuat.",
        data,
    ))
}

pub async fn get_viewport_places(
    State(state): State<AppState>,
    Query(params): Query<ViewportQueryParams>,
) -> AppResult<Json<ApiResponse<NearbyRadarResponseDto>>> {
    let data = radar_discovery_service::query_viewport(&state, &params).await?;
    Ok(ApiResponse::ok(
        "Area peta Beans Radar berhasil diperbarui.",
        data,
    ))
}

pub async fn search_coffee_places(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> AppResult<Json<ApiResponse<Vec<RadarPlaceCardDto>>>> {
    let data = radar_discovery_service::search_places(&state, &params).await?;
    Ok(ApiResponse::ok("Pencarian kedai kopi selesai.", data))
}

pub async fn get_radar_stats(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<RadarStatsDto>>> {
    let data = place_detail_service::get_stats(&state).await?;
    Ok(ApiResponse::ok("Statistik Beans Radar Indonesia berhasil dimuat.", data))
}
