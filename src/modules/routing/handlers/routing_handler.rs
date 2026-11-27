use axum::{
    extract::{Query, State},
    Json,
};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        AppState,
    },
    modules::routing::{
        dto::{
            CompareModesRequest, CompareModesResponseDto, ModeRoutesBundleDto, NavigateRequest,
            ReverseGeocodeQuery, ReverseGeocodeResponseDto,
        },
        error::validate_coord_pair,
        services::multimodal_router_service,
    },
};

pub async fn compare_modes(
    State(state): State<AppState>,
    Json(req): Json<CompareModesRequest>,
) -> AppResult<Json<ApiResponse<CompareModesResponseDto>>> {
    let (olat, olng) = validate_coord_pair(req.origin_lat, req.origin_lng)?;
    let (dlat, dlng) = validate_coord_pair(req.dest_lat, req.dest_lng)?;
    let avoid_tolls = req.avoid_tolls.unwrap_or(false);
    let alternates = req.alternates.unwrap_or(2).min(2);

    let data = multimodal_router_service::compare_all_modes(
        &state,
        [olat, olng],
        [dlat, dlng],
        avoid_tolls,
        alternates,
    )
    .await;

    Ok(ApiResponse::ok(
        "Perbandingan rute multi-modal berhasil dihitung.",
        data,
    ))
}

pub async fn navigate_live(
    State(state): State<AppState>,
    Json(req): Json<NavigateRequest>,
) -> AppResult<Json<ApiResponse<ModeRoutesBundleDto>>> {
    let (olat, olng) = validate_coord_pair(req.origin_lat, req.origin_lng)?;
    let (dlat, dlng) = validate_coord_pair(req.dest_lat, req.dest_lng)?;
    let avoid_tolls = req.avoid_tolls.unwrap_or(false);

    let bundle = multimodal_router_service::resolve_mode_bundle(
        &state,
        [olat, olng],
        [dlat, dlng],
        req.mode,
        avoid_tolls,
        req.bearing_deg,
        2,
    )
    .await;

    Ok(ApiResponse::ok(
        "Rute navigasi aktif berhasil diperbarui.",
        bundle,
    ))
}

pub async fn reverse_geocode(
    State(state): State<AppState>,
    Query(q): Query<ReverseGeocodeQuery>,
) -> AppResult<Json<ApiResponse<ReverseGeocodeResponseDto>>> {
    let (lat, lng) = validate_coord_pair(q.lat, q.lng)?;
    let info = multimodal_router_service::resolve_reverse_geocode(&state, lat, lng).await;
    Ok(ApiResponse::ok(
        "Lokasi kecamatan & kota berhasil diselesaikan.",
        info,
    ))
}
