use std::time::Instant;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::beans_radar::{
        dto::{
            NearbyQueryParams, NearbyRadarResponseDto, RadarPlaceCardDto, SearchQueryParams,
            ViewportQueryParams,
        },
        error::BeansRadarError,
        repositories::radar_place_repo,
        services::{
            h3_cache_service::H3CacheService,
            place_filter_rank::{build_filter, filter_post_hydrated, sort_cards},
            place_mapper::to_place_card_dto,
        },
        spatial_math::validate_coordinates,
    },
};

const DEFAULT_JAKARTA_LAT: f64 = -6.1944;
const DEFAULT_JAKARTA_LNG: f64 = 106.8229;

pub async fn query_nearby(
    state: &AppState,
    params: &NearbyQueryParams,
) -> AppResult<NearbyRadarResponseDto> {
    let start = Instant::now();
    let lat = params.lat.unwrap_or(DEFAULT_JAKARTA_LAT);
    let lng = params.lng.unwrap_or(DEFAULT_JAKARTA_LNG);
    validate_coordinates(lat, lng).map_err(BeansRadarError::InvalidCoordinates)?;

    let radius_km = params.radius_km.unwrap_or(10.0);
    if !(0.5..=500.0).contains(&radius_km) {
        return Err(BeansRadarError::RadiusOutOfBounds(radius_km).into());
    }
    let limit = params.limit.unwrap_or(60).clamp(1, 200);
    let candidate_cap = (limit * 3).min(450);

    let filter = build_filter(params.category.as_deref(), params.min_rating, params.sells_beans);
    let total_indexed = state.caches.beans_radar_rtree.loaded_nodes_count();
    let mut candidates = state
        .caches
        .beans_radar_rtree
        .locate_within_radius(lat, lng, radius_km, &filter, candidate_cap);

    // Jika radius lokal sangat sepi (misal luar Jawa) namun index berisi data nasional, perluas dengan KNN terdekat.
    if candidates.is_empty() && total_indexed > 0 && radius_km >= 100.0 {
        candidates = state.caches.beans_radar_rtree.knn_nearest(lat, lng, &filter, limit);
    }

    let (rows, cache_tier) = if !candidates.is_empty() {
        let ids: Vec<Uuid> = candidates.iter().map(|c| c.place_id).collect();
        H3CacheService::hydrate_ordered_places(
            &state.caches.radar_place_l1,
            state.redis.active_pool(),
            &state.pool,
            &ids,
        )
        .await?
    } else if total_indexed == 0 {
        let db_rows = radar_place_repo::query_postgis_nearby_fallback(
            &state.pool,
            lat,
            lng,
            radius_km * 1000.0,
            candidate_cap as i64,
        )
        .await?;
        (db_rows, "L3_POSTGIS_DIRECT")
    } else {
        (Vec::new(), "L1_RTREE_EMPTY_RADIUS")
    };

    let mut cards = filter_post_hydrated(
        rows,
        lat,
        lng,
        params.open_now,
        params.search.as_deref(),
        params.category.as_deref(),
        params.min_rating,
        params.sells_beans,
    );
    sort_cards(&mut cards, params.sort.as_deref());
    let total_matched = cards.len();
    cards.truncate(limit);

    Ok(NearbyRadarResponseDto {
        center_lat: lat,
        center_lng: lng,
        radius_km,
        query_h3_res7: H3CacheService::cell_res7_string(lat, lng),
        engine_latency_us: start.elapsed().as_micros() as u64,
        cache_tier: cache_tier.to_string(),
        total_indexed_places: total_indexed,
        total_matched,
        items: cards,
    })
}

pub async fn query_viewport(
    state: &AppState,
    params: &ViewportQueryParams,
) -> AppResult<NearbyRadarResponseDto> {
    let start = Instant::now();
    let center_lat = (params.north + params.south) * 0.5;
    let center_lng = (params.east + params.west) * 0.5;
    validate_coordinates(center_lat, center_lng).map_err(BeansRadarError::InvalidCoordinates)?;

    let limit = params.limit.unwrap_or(100).clamp(1, 250);
    let rows = radar_place_repo::query_viewport_places(
        &state.pool,
        params.north.max(params.south),
        params.south.min(params.north),
        params.east.max(params.west),
        params.west.min(params.east),
        (limit * 2) as i64,
    )
    .await?;

    let mut cards = filter_post_hydrated(
        rows,
        center_lat,
        center_lng,
        params.open_now,
        None,
        params.category.as_deref(),
        params.min_rating,
        params.sells_beans,
    );
    cards.sort_by(|a, b| b.bayesian_score.total_cmp(&a.bayesian_score));
    let total_matched = cards.len();
    cards.truncate(limit);

    Ok(NearbyRadarResponseDto {
        center_lat,
        center_lng,
        radius_km: 0.0,
        query_h3_res7: H3CacheService::cell_res7_string(center_lat, center_lng),
        engine_latency_us: start.elapsed().as_micros() as u64,
        cache_tier: "L3_POSTGIS_VIEWPORT".to_string(),
        total_indexed_places: state.caches.beans_radar_rtree.loaded_nodes_count(),
        total_matched,
        items: cards,
    })
}

pub async fn search_places(
    state: &AppState,
    params: &SearchQueryParams,
) -> AppResult<Vec<RadarPlaceCardDto>> {
    let lat = params.lat.unwrap_or(DEFAULT_JAKARTA_LAT);
    let lng = params.lng.unwrap_or(DEFAULT_JAKARTA_LNG);
    validate_coordinates(lat, lng).map_err(BeansRadarError::InvalidCoordinates)?;
    let limit = params.limit.unwrap_or(40).clamp(1, 100) as i64;
    let rows = radar_place_repo::search_places_trgm(&state.pool, &params.q, limit).await?;
    let mut cards: Vec<RadarPlaceCardDto> = rows.iter().map(|r| to_place_card_dto(r, lat, lng)).collect();
    cards.sort_by(|a, b| a.distance_meters.total_cmp(&b.distance_meters));
    Ok(cards)
}
