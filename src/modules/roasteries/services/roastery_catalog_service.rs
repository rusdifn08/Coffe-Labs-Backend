use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::core::{
    error::{AppError, AppResult},
    storage::auto_optimize_url,
    AppState,
};
use crate::modules::roasteries::{
    dto::PublicRoasteryQuery,
    models::{RoasteryLineupRow, RoasteryRow, RoasteryWithLineupsResponse},
    repositories::{lineup_repo, roastery_repo},
};

pub const CATALOG_CACHE_KEY_DEFAULT: &str = "roasteries:public:default";

pub fn optimize_lineup_asset(mut row: RoasteryLineupRow) -> RoasteryLineupRow {
    if let Some(ref url) = row.bag_image_url {
        row.bag_image_url = Some(auto_optimize_url(url));
    }
    row
}

pub fn assemble_response(
    mut roastery: RoasteryRow,
    lineups: Vec<RoasteryLineupRow>,
) -> RoasteryWithLineupsResponse {
    roastery.logo_url = auto_optimize_url(&roastery.logo_url);
    if let Some(ref banner) = roastery.banner_url {
        roastery.banner_url = Some(auto_optimize_url(banner));
    }
    let optimized_lineups = lineups.into_iter().map(optimize_lineup_asset).collect();
    RoasteryWithLineupsResponse { roastery, lineups: optimized_lineups }
}

pub async fn list_public_roasteries(
    state: &AppState,
    query: &PublicRoasteryQuery,
) -> AppResult<Vec<RoasteryWithLineupsResponse>> {
    let is_default_cacheable = query.cluster.as_deref().map(str::trim).unwrap_or("").is_empty()
        && query.q.as_deref().map(str::trim).unwrap_or("").is_empty()
        && query.offset.unwrap_or(0) == 0
        && query.limit.unwrap_or(50) == 50;

    if is_default_cacheable {
        if let Some(cached) = state.caches.directory_l1.get(CATALOG_CACHE_KEY_DEFAULT).await {
            if let Ok(parsed) = serde_json::from_value::<Vec<RoasteryWithLineupsResponse>>((*cached).clone()) {
                return Ok(parsed);
            }
        }
    }

    let roasteries = roastery_repo::list_published(
        &state.pool,
        query.cluster.as_deref(),
        query.q.as_deref(),
        query.limit.unwrap_or(50),
        query.offset.unwrap_or(0),
    )
    .await?;

    let ids: Vec<Uuid> = roasteries.iter().map(|r| r.id).collect();
    let all_lineups = lineup_repo::list_for_roasteries_batch(&state.pool, &ids).await?;

    let mut grouped: HashMap<Uuid, Vec<RoasteryLineupRow>> = HashMap::new();
    for lineup in all_lineups {
        grouped.entry(lineup.roastery_id).or_default().push(lineup);
    }

    let result: Vec<RoasteryWithLineupsResponse> = roasteries
        .into_iter()
        .map(|r| {
            let lineups = grouped.remove(&r.id).unwrap_or_default();
            assemble_response(r, lineups)
        })
        .collect();

    if is_default_cacheable {
        if let Ok(val) = serde_json::to_value(&result) {
            state
                .caches
                .directory_l1
                .insert(CATALOG_CACHE_KEY_DEFAULT.to_string(), Arc::new(val))
                .await;
        }
    }

    Ok(result)
}

pub async fn get_public_roastery_detail(
    state: &AppState,
    id_or_slug: &str,
) -> AppResult<RoasteryWithLineupsResponse> {
    let roastery = roastery_repo::find_published_by_id_or_slug(&state.pool, id_or_slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Roastery tidak ditemukan atau belum dipublikasikan.".to_string()))?;
    let lineups = lineup_repo::list_for_roastery(&state.pool, roastery.id).await?;
    Ok(assemble_response(roastery, lineups))
}
