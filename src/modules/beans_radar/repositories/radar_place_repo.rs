use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::beans_radar::models::{CoffeeRadarPlaceRow, CompactNodeDbRow, RoasteryLineupPreviewRow},
};

const PLACE_SELECT_COLS: &str = r#"
    id, external_source, linked_roastery_id, slug, name,
    category::text AS category, sub_categories, latitude, longitude,
    h3_res7, h3_res9, full_address, district_kecamatan, city_kabupaten,
    province, timezone, rating, reviews_count, bayesian_score, price_level,
    opening_hours_json, is_24_hours, sells_roast_beans, has_manual_brew_v60,
    featured_bean_summary, phone_number, whatsapp_number, instagram_handle,
    website_url, google_maps_url, thumbnail_url, is_verified_partner,
    last_verified_at
"#;

pub async fn load_all_compact_nodes(pool: &PgPool) -> AppResult<Vec<CompactNodeDbRow>> {
    let rows = sqlx::query_as::<_, CompactNodeDbRow>(
        r#"SELECT id, latitude, longitude, category::text AS category,
                  rating, bayesian_score, sells_roast_beans, has_manual_brew_v60, is_verified_partner
           FROM coffee_radar_places
           WHERE is_active = TRUE"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn batch_hydrate_by_ids(
    pool: &PgPool,
    ids: &[Uuid],
) -> AppResult<Vec<CoffeeRadarPlaceRow>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let query_str = format!(
        "SELECT {} FROM coffee_radar_places WHERE id = ANY($1) AND is_active = TRUE",
        PLACE_SELECT_COLS
    );
    let rows = sqlx::query_as::<_, CoffeeRadarPlaceRow>(&query_str)
        .bind(ids)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn query_viewport_places(
    pool: &PgPool,
    north: f64,
    south: f64,
    east: f64,
    west: f64,
    limit: i64,
) -> AppResult<Vec<CoffeeRadarPlaceRow>> {
    let query_str = format!(
        r#"SELECT {} FROM coffee_radar_places
           WHERE is_active = TRUE
             AND latitude BETWEEN $1 AND $2
             AND longitude BETWEEN $3 AND $4
           ORDER BY bayesian_score DESC, reviews_count DESC
           LIMIT $5"#,
        PLACE_SELECT_COLS
    );
    let rows = sqlx::query_as::<_, CoffeeRadarPlaceRow>(&query_str)
        .bind(south)
        .bind(north)
        .bind(west)
        .bind(east)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn query_postgis_nearby_fallback(
    pool: &PgPool,
    lat: f64,
    lng: f64,
    radius_meters: f64,
    limit: i64,
) -> AppResult<Vec<CoffeeRadarPlaceRow>> {
    let query_str = format!(
        r#"SELECT {} FROM coffee_radar_places
           WHERE is_active = TRUE
             AND ST_DWithin(
                 location,
                 ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                 $3
             )
           ORDER BY location <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
           LIMIT $4"#,
        PLACE_SELECT_COLS
    );
    let rows = sqlx::query_as::<_, CoffeeRadarPlaceRow>(&query_str)
        .bind(lat)
        .bind(lng)
        .bind(radius_meters)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn search_places_trgm(
    pool: &PgPool,
    query: &str,
    limit: i64,
) -> AppResult<Vec<CoffeeRadarPlaceRow>> {
    let like_pattern = format!("%{}%", query.trim());
    let query_str = format!(
        r#"SELECT {} FROM coffee_radar_places
           WHERE is_active = TRUE
             AND (name ILIKE $1 OR full_address ILIKE $1 OR city_kabupaten ILIKE $1 OR district_kecamatan ILIKE $1)
           ORDER BY similarity(name, $2) DESC, bayesian_score DESC
           LIMIT $3"#,
        PLACE_SELECT_COLS
    );
    let rows = sqlx::query_as::<_, CoffeeRadarPlaceRow>(&query_str)
        .bind(&like_pattern)
        .bind(query.trim())
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn fetch_place_by_id(pool: &PgPool, place_id: Uuid) -> AppResult<Option<CoffeeRadarPlaceRow>> {
    let query_str = format!(
        "SELECT {} FROM coffee_radar_places WHERE id = $1 AND is_active = TRUE",
        PLACE_SELECT_COLS
    );
    let row = sqlx::query_as::<_, CoffeeRadarPlaceRow>(&query_str)
        .bind(place_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn fetch_linked_roastery_lineups(
    pool: &PgPool,
    roastery_id: Uuid,
) -> AppResult<Vec<RoasteryLineupPreviewRow>> {
    let rows = sqlx::query_as::<_, RoasteryLineupPreviewRow>(
        r#"SELECT id, name, origin, roast_profile, process, tasting_notes,
                  price_100g_label, price_200g_label, bag_image_url, badge
           FROM roastery_coffee_lineups
           WHERE roastery_id = $1 AND is_available = TRUE
           ORDER BY display_order ASC
           LIMIT 12"#,
    )
    .bind(roastery_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
