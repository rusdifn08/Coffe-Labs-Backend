use sqlx::{PgPool, Row};

use crate::{
    core::error::AppResult,
    modules::beans_radar::dto::{RadarCategoryCountDto, RadarCityCountDto},
};

pub async fn fetch_stats_aggregates(
    pool: &PgPool,
) -> AppResult<(i64, i64, i64, Vec<RadarCategoryCountDto>, Vec<RadarCityCountDto>)> {
    let summary = sqlx::query(
        r#"SELECT COUNT(*) AS total,
                  COUNT(DISTINCT NULLIF(city_kabupaten, '')) AS cities,
                  COUNT(*) FILTER (WHERE is_verified_partner = TRUE) AS partners
           FROM coffee_radar_places
           WHERE is_active = TRUE"#,
    )
    .fetch_one(pool)
    .await?;

    let cats = sqlx::query(
        r#"SELECT category::text AS cat, COUNT(*) AS cnt
           FROM coffee_radar_places
           WHERE is_active = TRUE
           GROUP BY category
           ORDER BY cnt DESC"#,
    )
    .fetch_all(pool)
    .await?;

    let cities = sqlx::query(
        r#"SELECT city_kabupaten AS city, COUNT(*) AS cnt
           FROM coffee_radar_places
           WHERE is_active = TRUE AND city_kabupaten <> ''
           GROUP BY city_kabupaten
           ORDER BY cnt DESC
           LIMIT 15"#,
    )
    .fetch_all(pool)
    .await?;

    let by_category = cats
        .into_iter()
        .map(|r| RadarCategoryCountDto {
            category: r.try_get("cat").unwrap_or_default(),
            count: r.try_get("cnt").unwrap_or(0),
        })
        .collect();
    let top_cities = cities
        .into_iter()
        .map(|r| RadarCityCountDto {
            city: r.try_get("city").unwrap_or_default(),
            count: r.try_get("cnt").unwrap_or(0),
        })
        .collect();

    Ok((
        summary.try_get("total").unwrap_or(0),
        summary.try_get("cities").unwrap_or(0),
        summary.try_get("partners").unwrap_or(0),
        by_category,
        top_cities,
    ))
}
