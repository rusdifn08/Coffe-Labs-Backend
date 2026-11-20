use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::radar::models::{NearbyRadarPlaceDto, RadarPlaceKind, RadarScopeMode, RadarStatsDto},
};

pub fn build_kind_filter_sql(kind: Option<RadarPlaceKind>, mode: RadarScopeMode) -> Vec<&'static str> {
    if let Some(k) = kind {
        return vec![k.as_db_str()];
    }
    match mode {
        RadarScopeMode::Cafe => vec!["CAFE", "ROASTERY_AND_CAFE"],
        RadarScopeMode::Beans => vec!["BEAN_SELLER", "ROASTERY_AND_CAFE"],
        RadarScopeMode::All => vec!["CAFE", "BEAN_SELLER", "ROASTERY_AND_CAFE"],
    }
}

fn map_row(row: &sqlx::postgres::PgRow) -> NearbyRadarPlaceDto {
    let kind: String = row.try_get("kind").unwrap_or_else(|_| "CAFE".to_string());
    let dist_m: f64 = row.try_get::<f64, _>("distance_meters").unwrap_or(0.0).max(0.0);
    let eta_walk = ((dist_m / 75.0).ceil() as i32).max(1);
    let eta_drive = ((dist_m / 450.0).ceil() as i32).max(1);
    let sells_beans = kind == "BEAN_SELLER" || kind == "ROASTERY_AND_CAFE";
    let serves_cafe = kind == "CAFE" || kind == "ROASTERY_AND_CAFE";

    NearbyRadarPlaceDto {
        id: row.try_get("id").unwrap_or_else(|_| Uuid::nil()),
        linked_roastery_id: row.try_get("linked_roastery_id").ok().flatten(),
        name: row.try_get("name").unwrap_or_default(),
        kind,
        category_tag: row.try_get("category_tag").unwrap_or_else(|_| "COFFEE_SHOP".into()),
        address: row.try_get("address").unwrap_or_default(),
        district: row.try_get("district").unwrap_or_default(),
        city: row.try_get("city").unwrap_or_default(),
        province: row.try_get("province").unwrap_or_default(),
        timezone: row.try_get("timezone").unwrap_or_else(|_| "Asia/Jakarta".into()),
        latitude: row.try_get("latitude").unwrap_or(0.0),
        longitude: row.try_get("longitude").unwrap_or(0.0),
        distance_meters: (dist_m * 10.0).round() / 10.0,
        eta_walking_min: eta_walk,
        eta_driving_min: eta_drive,
        rating: row.try_get("rating").unwrap_or(4.5),
        reviews_count: row.try_get("reviews_count").unwrap_or(0),
        sells_beans,
        serves_cafe,
        featured_beans: row.try_get("featured_beans").unwrap_or_else(|_| json!([])),
        brewing_methods: row.try_get("brewing_methods").unwrap_or_else(|_| json!([])),
        phone: row.try_get("phone").ok().flatten(),
        whatsapp_number: row.try_get("whatsapp_number").ok().flatten(),
        instagram_handle: row.try_get("instagram_handle").ok().flatten(),
        opening_hours: row.try_get("opening_hours").unwrap_or_else(|_| json!({})),
        image_url: row.try_get("image_url").ok().flatten(),
        is_verified: row.try_get("is_verified").unwrap_or(false),
    }
}

pub async fn query_nearby_places(
    pool: &PgPool,
    lat: f64,
    lng: f64,
    radius_km: f64,
    kind: Option<RadarPlaceKind>,
    mode: RadarScopeMode,
    search: Option<&str>,
    limit: i64,
) -> AppResult<Vec<NearbyRadarPlaceDto>> {
    let kinds: Vec<String> = build_kind_filter_sql(kind, mode)
        .into_iter()
        .map(String::from)
        .collect();
    let search_pattern = search
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{s}%"));

    let rows = sqlx::query(
        r#"
        SELECT
            id, linked_roastery_id, name, kind::text AS kind, category_tag,
            address, district, city, province, timezone, latitude, longitude,
            ST_Distance(location, ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography) AS distance_meters,
            rating, reviews_count, featured_beans, brewing_methods,
            phone, whatsapp_number, instagram_handle, opening_hours, image_url, is_verified
        FROM radar_places
        WHERE is_active = TRUE
          AND kind::text = ANY($4)
          AND ST_DWithin(location, ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography, $3 * 1000.0)
          AND (
              $5::text IS NULL OR name ILIKE $5 OR city ILIKE $5
              OR address ILIKE $5 OR featured_beans::text ILIKE $5
          )
        ORDER BY distance_meters ASC
        LIMIT $6
        "#,
    )
    .bind(lng)
    .bind(lat)
    .bind(radius_km)
    .bind(&kinds)
    .bind(search_pattern)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_row).collect())
}

pub async fn query_viewport_places(
    pool: &PgPool,
    north: f64,
    south: f64,
    east: f64,
    west: f64,
    kind: Option<RadarPlaceKind>,
    mode: RadarScopeMode,
    limit: i64,
) -> AppResult<Vec<NearbyRadarPlaceDto>> {
    let kinds: Vec<String> = build_kind_filter_sql(kind, mode)
        .into_iter()
        .map(String::from)
        .collect();
    let c_lat = (north + south) / 2.0;
    let c_lng = (east + west) / 2.0;

    let rows = sqlx::query(
        r#"
        SELECT
            id, linked_roastery_id, name, kind::text AS kind, category_tag,
            address, district, city, province, timezone, latitude, longitude,
            ST_Distance(location, ST_SetSRID(ST_MakePoint($5, $6), 4326)::geography) AS distance_meters,
            rating, reviews_count, featured_beans, brewing_methods,
            phone, whatsapp_number, instagram_handle, opening_hours, image_url, is_verified
        FROM radar_places
        WHERE is_active = TRUE
          AND kind::text = ANY($7)
          AND location::geometry && ST_MakeEnvelope($1, $2, $3, $4, 4326)
        ORDER BY is_verified DESC, reviews_count DESC, distance_meters ASC
        LIMIT $8
        "#,
    )
    .bind(west)
    .bind(south)
    .bind(east)
    .bind(north)
    .bind(c_lng)
    .bind(c_lat)
    .bind(&kinds)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_row).collect())
}

pub async fn lookup_place_coords(pool: &PgPool, place_id: Uuid) -> AppResult<Option<(f64, f64)>> {
    let row = sqlx::query(
        "SELECT latitude, longitude FROM radar_places WHERE id = $1 AND is_active = TRUE",
    )
    .bind(place_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.and_then(|r| {
        let lat: f64 = r.try_get("latitude").ok()?;
        let lng: f64 = r.try_get("longitude").ok()?;
        Some((lat, lng))
    }))
}

pub async fn fetch_radar_stats(pool: &PgPool) -> AppResult<RadarStatsDto> {
    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*)::bigint AS total_places,
            COUNT(*) FILTER (WHERE kind IN ('CAFE', 'ROASTERY_AND_CAFE'))::bigint AS cafe_radar_total,
            COUNT(*) FILTER (WHERE kind IN ('BEAN_SELLER', 'ROASTERY_AND_CAFE'))::bigint AS beans_radar_total,
            COUNT(*) FILTER (WHERE kind = 'ROASTERY_AND_CAFE')::bigint AS dual_roastery_cafe_total,
            COUNT(DISTINCT NULLIF(city, ''))::bigint AS cities_count,
            COUNT(DISTINCT NULLIF(province, ''))::bigint AS provinces_count
        FROM radar_places
        WHERE is_active = TRUE
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(RadarStatsDto {
        total_places: row.try_get("total_places").unwrap_or(0),
        cafe_radar_total: row.try_get("cafe_radar_total").unwrap_or(0),
        beans_radar_total: row.try_get("beans_radar_total").unwrap_or(0),
        dual_roastery_cafe_total: row.try_get("dual_roastery_cafe_total").unwrap_or(0),
        cities_count: row.try_get("cities_count").unwrap_or(0),
        provinces_count: row.try_get("provinces_count").unwrap_or(0),
    })
}
