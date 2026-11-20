use crate::modules::beans_radar::{
    dto::{RadarLineupPreviewDto, RadarPlaceCardDto, RadarPlaceDetailDto},
    models::{CoffeeRadarPlaceRow, RoasteryLineupPreviewRow},
    opening_hours_eval::evaluate_open_status_now,
    spatial_math::{estimate_eta_minutes, format_distance_label, haversine_meters},
};

pub fn to_place_card_dto(
    row: &CoffeeRadarPlaceRow,
    origin_lat: f64,
    origin_lng: f64,
) -> RadarPlaceCardDto {
    let dist_m = haversine_meters(origin_lat, origin_lng, row.latitude, row.longitude);
    let (walk_eta, ride_eta) = estimate_eta_minutes(dist_m);
    let sub_categories = row
        .sub_categories
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    RadarPlaceCardDto {
        id: row.id,
        slug: row.slug.clone(),
        linked_roastery_id: row.linked_roastery_id,
        name: row.name.clone(),
        category: row.category.clone(),
        sub_categories,
        latitude: row.latitude,
        longitude: row.longitude,
        h3_res7: format!("{:x}", row.h3_res7 as u64),
        full_address: row.full_address.clone(),
        district_kecamatan: row.district_kecamatan.clone(),
        city_kabupaten: row.city_kabupaten.clone(),
        province: row.province.clone(),
        rating: row.rating,
        reviews_count: row.reviews_count,
        bayesian_score: row.bayesian_score,
        price_level: row.price_level.unwrap_or(2),
        distance_meters: (dist_m * 10.0).round() / 10.0,
        distance_label: format_distance_label(dist_m),
        walk_eta_min: walk_eta,
        ride_eta_min: ride_eta,
        open_status: evaluate_open_status_now(
            &row.opening_hours_json,
            row.is_24_hours,
            &row.timezone,
        ),
        sells_roast_beans: row.sells_roast_beans,
        has_manual_brew_v60: row.has_manual_brew_v60,
        featured_bean_summary: row.featured_bean_summary.clone(),
        google_maps_url: row.google_maps_url.clone(),
        thumbnail_url: row.thumbnail_url.clone(),
        is_verified_partner: row.is_verified_partner,
    }
}

pub fn to_place_detail_dto(
    row: &CoffeeRadarPlaceRow,
    origin_lat: f64,
    origin_lng: f64,
    is_favorite: bool,
    lineups: Vec<RoasteryLineupPreviewRow>,
) -> RadarPlaceDetailDto {
    RadarPlaceDetailDto {
        card: to_place_card_dto(row, origin_lat, origin_lng),
        timezone: row.timezone.clone(),
        is_24_hours: row.is_24_hours,
        opening_hours: row.opening_hours_json.clone(),
        phone_number: row.phone_number.clone(),
        whatsapp_number: row.whatsapp_number.clone(),
        instagram_handle: row.instagram_handle.clone(),
        website_url: row.website_url.clone(),
        is_favorite,
        linked_roastery_lineups: lineups
            .into_iter()
            .map(|l| RadarLineupPreviewDto {
                id: l.id,
                name: l.name,
                origin: l.origin,
                roast_profile: l.roast_profile,
                process: l.process,
                tasting_notes: l.tasting_notes,
                price_100g_label: l.price_100g_label,
                price_200g_label: l.price_200g_label,
                bag_image_url: l.bag_image_url,
                badge: l.badge,
            })
            .collect(),
    }
}
