use axum::{
    extract::{Query, State},
    Json,
};

use crate::{
    core::{
        auth_extractor::AuthenticatedUser,
        error::{ApiResponse, AppError, AppResult},
        AppState,
    },
    modules::radar::{
        bulk_import_repo,
        dto::{
            normalize_route_mode, parse_origin_coords, validate_coords, BulkImportRequest,
            NearbyRadarQuery, RouteQuery, ViewportRadarQuery,
        },
        models::{
            BulkImportResultDto, NearbyRadarPlaceDto, RadarPlaceKind, RadarScopeMode,
            RadarStatsDto, RoutePolylineDto,
        },
        repository, routing_service,
    },
};

pub async fn get_nearby_places(
    State(state): State<AppState>,
    Query(q): Query<NearbyRadarQuery>,
) -> AppResult<Json<ApiResponse<Vec<NearbyRadarPlaceDto>>>> {
    let (lat, lng) = validate_coords(q.lat, q.lng)?;
    let radius_km = q.radius_km.unwrap_or(10.0).clamp(0.5, 150.0);
    let limit = q.limit.unwrap_or(40).clamp(1, 120);
    let kind = RadarPlaceKind::parse_opt(q.kind.as_deref());
    let mode = RadarScopeMode::parse(q.mode.as_deref());

    let items = repository::query_nearby_places(
        &state.pool,
        lat,
        lng,
        radius_km,
        kind,
        mode,
        q.search.as_deref(),
        limit,
    )
    .await?;

    Ok(ApiResponse::ok("Lokasi Coffee & Beans Radar terdekat berhasil dimuat.", items))
}

pub async fn get_viewport_places(
    State(state): State<AppState>,
    Query(q): Query<ViewportRadarQuery>,
) -> AppResult<Json<ApiResponse<Vec<NearbyRadarPlaceDto>>>> {
    validate_coords(q.north, q.east)?;
    validate_coords(q.south, q.west)?;
    if q.north <= q.south || q.east <= q.west {
        return Err(AppError::Validation("Bounding box viewport peta tidak valid.".into()));
    }
    let limit = q.limit.unwrap_or(80).clamp(1, 200);
    let kind = RadarPlaceKind::parse_opt(q.kind.as_deref());
    let mode = RadarScopeMode::parse(q.mode.as_deref());

    let items = repository::query_viewport_places(
        &state.pool,
        q.north,
        q.south,
        q.east,
        q.west,
        kind,
        mode,
        limit,
    )
    .await?;

    Ok(ApiResponse::ok("Pin area peta berhasil dimuat.", items))
}

pub async fn get_route_polyline(
    State(state): State<AppState>,
    Query(q): Query<RouteQuery>,
) -> AppResult<Json<ApiResponse<RoutePolylineDto>>> {
    let (o_lat, o_lng) = parse_origin_coords(&q)?;
    let mode = normalize_route_mode(q.mode.as_deref());

    let dest_uuid = q.dest_place_id.or(q.dest_id);
    let (d_lat, d_lng, dest_key) = if let Some(pid) = dest_uuid {
        let coords = repository::lookup_place_coords(&state.pool, pid)
            .await?
            .ok_or_else(|| AppError::NotFound("Lokasi tujuan tidak ditemukan di radar.".into()))?;
        (coords.0, coords.1, pid.to_string())
    } else if let (Some(lat), Some(lng)) = (q.dest_lat, q.dest_lng) {
        let (vlat, vlng) = validate_coords(lat, lng)?;
        (vlat, vlng, format!("{vlat:.4},{vlng:.4}"))
    } else {
        return Err(AppError::Validation(
            "Parameter dest_place_id atau dest_lat & dest_lng wajib diisi.".into(),
        ));
    };

    let route = routing_service::resolve_route_with_cache(
        &state.redis,
        state.config.geoapify_api_key.as_deref(),
        [o_lat, o_lng],
        [d_lat, d_lng],
        &dest_key,
        mode,
    )
    .await?;

    Ok(ApiResponse::ok("Rute jalan berhasil dihitung.", route))
}

pub async fn get_radar_stats(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<RadarStatsDto>>> {
    let stats = repository::fetch_radar_stats(&state.pool).await?;
    Ok(ApiResponse::ok("Statistik 10.000+ lokasi Beans & Cafe Radar.", stats))
}

pub async fn bulk_import_places(
    user: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<BulkImportRequest>,
) -> AppResult<Json<ApiResponse<BulkImportResultDto>>> {
    if !user.claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::Forbidden(
            "Hanya admin Coffee Labs yang dapat menjalankan bulk import radar.".into(),
        ));
    }
    if req.places.is_empty() || req.places.len() > 2500 {
        return Err(AppError::Validation(
            "Jumlah item bulk import harus antara 1 sampai 2500 per batch.".into(),
        ));
    }
    let result = bulk_import_repo::bulk_upsert_places(&state.pool, &req.places).await?;
    Ok(ApiResponse::ok("Bulk upsert lokasi radar selesai.", result))
}
