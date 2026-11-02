use sqlx::PgPool;

use crate::{core::error::AppResult, modules::beans::models::CuratedBeanRow};

pub async fn query_beans(
    pool: &PgPool,
    q: Option<&str>,
    roastery: Option<&str>,
    city: Option<&str>,
    process: Option<&str>,
) -> AppResult<Vec<CuratedBeanRow>> {
    let search_pattern = q.map(|s| format!("%{}%", s.trim()));
    let roastery_pattern = roastery.map(|s| format!("%{}%", s.trim()));
    let city_pattern = city.map(|s| format!("%{}%", s.trim()));
    let process_pattern = process.map(|s| format!("%{}%", s.trim()));

    let rows = sqlx::query_as::<_, CuratedBeanRow>(
        r#"SELECT * FROM curated_beans
           WHERE ($1::text IS NULL OR name ILIKE $1 OR tasting_notes ILIKE $1 OR origin ILIKE $1)
             AND ($2::text IS NULL OR roastery_name ILIKE $2)
             AND ($3::text IS NULL OR roastery_city ILIKE $3)
             AND ($4::text IS NULL OR process ILIKE $4)
           ORDER BY rating DESC, name ASC
           LIMIT 100"#,
    )
    .bind(search_pattern)
    .bind(roastery_pattern)
    .bind(city_pattern)
    .bind(process_pattern)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
