use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::beans_radar::{
        dto::{RadarPlaceCardDto, RadarPlaceDetailDto, RadarStatsDto, ToggleFavoriteResponseDto},
        error::BeansRadarError,
        repositories::{favorite_place_repo, radar_place_repo, radar_stats_repo},
        services::place_mapper::{to_place_card_dto, to_place_detail_dto},
    },
};

const DEFAULT_JAKARTA_LAT: f64 = -6.1944;
const DEFAULT_JAKARTA_LNG: f64 = 106.8229;

pub async fn get_place_detail(
    state: &AppState,
    place_id: Uuid,
    viewer_user_id: Option<Uuid>,
    lat: Option<f64>,
    lng: Option<f64>,
) -> AppResult<RadarPlaceDetailDto> {
    let row = radar_place_repo::fetch_place_by_id(&state.pool, place_id)
        .await?
        .ok_or(BeansRadarError::PlaceNotFound)?;
    let origin_lat = lat.unwrap_or(row.latitude);
    let origin_lng = lng.unwrap_or(row.longitude);

    let is_fav = match viewer_user_id {
        Some(uid) => favorite_place_repo::is_place_favorited(&state.pool, uid, place_id).await?,
        None => false,
    };
    let lineups = match row.linked_roastery_id {
        Some(rid) => radar_place_repo::fetch_linked_roastery_lineups(&state.pool, rid).await?,
        None => Vec::new(),
    };

    Ok(to_place_detail_dto(&row, origin_lat, origin_lng, is_fav, lineups))
}

pub async fn toggle_favorite(
    state: &AppState,
    user_id: Uuid,
    place_id: Uuid,
) -> AppResult<ToggleFavoriteResponseDto> {
    let _ = radar_place_repo::fetch_place_by_id(&state.pool, place_id)
        .await?
        .ok_or(BeansRadarError::PlaceNotFound)?;
    let is_fav = favorite_place_repo::toggle_favorite(&state.pool, user_id, place_id).await?;
    Ok(ToggleFavoriteResponseDto {
        place_id,
        is_favorite: is_fav,
    })
}

pub async fn list_favorites(
    state: &AppState,
    user_id: Uuid,
    lat: Option<f64>,
    lng: Option<f64>,
) -> AppResult<Vec<RadarPlaceCardDto>> {
    let origin_lat = lat.unwrap_or(DEFAULT_JAKARTA_LAT);
    let origin_lng = lng.unwrap_or(DEFAULT_JAKARTA_LNG);
    let rows = favorite_place_repo::list_user_favorite_places(&state.pool, user_id, 100).await?;
    Ok(rows
        .iter()
        .map(|r| to_place_card_dto(r, origin_lat, origin_lng))
        .collect())
}

pub async fn get_stats(state: &AppState) -> AppResult<RadarStatsDto> {
    let (total_active_places, total_cities, verified_partners_count, by_category, top_cities) =
        radar_stats_repo::fetch_stats_aggregates(&state.pool).await?;
    Ok(RadarStatsDto {
        total_active_places,
        total_cities,
        verified_partners_count,
        rtree_loaded_nodes: state.caches.beans_radar_rtree.loaded_nodes_count(),
        by_category,
        top_cities,
    })
}
