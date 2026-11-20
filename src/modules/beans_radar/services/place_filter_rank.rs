use crate::modules::beans_radar::{
    dto::RadarPlaceCardDto,
    models::{CoffeeRadarPlaceRow, CompactSpatialNode},
    services::{place_mapper::to_place_card_dto, rtree_memory_engine::RTreeQueryFilter},
    spatial_math::compute_smart_discovery_score,
};

fn parse_category_filter(category: Option<&str>) -> Option<u8> {
    let clean = category?.trim().to_uppercase();
    if clean.is_empty() || clean == "ALL" || clean == "SEMUA" {
        return None;
    }
    Some(CompactSpatialNode::category_to_code(&clean))
}

pub fn build_filter(
    category: Option<&str>,
    min_rating: Option<f32>,
    sells_beans: Option<bool>,
) -> RTreeQueryFilter {
    let rating_clamped = min_rating.unwrap_or(0.0).clamp(0.0, 5.0);
    RTreeQueryFilter {
        category_code: parse_category_filter(category),
        min_rating_x10: (rating_clamped * 10.0).round() as u8,
        sells_beans_only: sells_beans.unwrap_or(false),
    }
}

pub fn sort_cards(cards: &mut [RadarPlaceCardDto], sort_mode: Option<&str>) {
    match sort_mode.unwrap_or("nearest").trim().to_lowercase().as_str() {
        "rating" => cards.sort_by(|a, b| {
            b.bayesian_score
                .total_cmp(&a.bayesian_score)
                .then_with(|| a.distance_meters.total_cmp(&b.distance_meters))
        }),
        "smart" | "recommended" => cards.sort_by(|a, b| {
            let sa = compute_smart_discovery_score(
                a.bayesian_score,
                a.distance_meters,
                a.open_status.is_open_now,
                a.is_verified_partner,
            );
            let sb = compute_smart_discovery_score(
                b.bayesian_score,
                b.distance_meters,
                b.open_status.is_open_now,
                b.is_verified_partner,
            );
            sb.total_cmp(&sa)
        }),
        _ => cards.sort_by(|a, b| a.distance_meters.total_cmp(&b.distance_meters)),
    }
}

pub fn filter_post_hydrated(
    rows: Vec<CoffeeRadarPlaceRow>,
    lat: f64,
    lng: f64,
    open_now: Option<bool>,
    search: Option<&str>,
    cat_filter: Option<&str>,
    min_rating: Option<f32>,
    sells_beans: Option<bool>,
) -> Vec<RadarPlaceCardDto> {
    let needle = search.unwrap_or("").trim().to_lowercase();
    let cat_upper = cat_filter.unwrap_or("").trim().to_uppercase();
    let check_cat = !cat_upper.is_empty() && cat_upper != "ALL" && cat_upper != "SEMUA";
    let min_r = min_rating.unwrap_or(0.0);
    let only_beans = sells_beans.unwrap_or(false);
    let only_open = open_now.unwrap_or(false);

    rows.into_iter()
        .filter(|r| !check_cat || r.category.to_uppercase() == cat_upper)
        .filter(|r| r.rating >= min_r)
        .filter(|r| !only_beans || r.sells_roast_beans)
        .filter(|r| {
            needle.is_empty()
                || r.name.to_lowercase().contains(&needle)
                || r.full_address.to_lowercase().contains(&needle)
                || r.city_kabupaten.to_lowercase().contains(&needle)
        })
        .map(|r| to_place_card_dto(&r, lat, lng))
        .filter(|c| !only_open || c.open_status.is_open_now)
        .collect()
}
