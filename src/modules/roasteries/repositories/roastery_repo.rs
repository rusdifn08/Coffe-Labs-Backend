use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::modules::roasteries::models::RoasteryRow;

pub async fn find_by_owner(pool: &PgPool, owner_user_id: Uuid) -> AppResult<Option<RoasteryRow>> {
    let row = sqlx::query_as::<_, RoasteryRow>(
        "SELECT * FROM coffee_roasteries WHERE owner_user_id = $1",
    )
    .bind(owner_user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn find_published_by_id_or_slug(
    pool: &PgPool,
    id_or_slug: &str,
) -> AppResult<Option<RoasteryRow>> {
    if let Ok(uuid) = Uuid::parse_str(id_or_slug.trim()) {
        let row = sqlx::query_as::<_, RoasteryRow>(
            "SELECT * FROM coffee_roasteries WHERE id = $1 AND is_published = TRUE",
        )
        .bind(uuid)
        .fetch_optional(pool)
        .await?;
        return Ok(row);
    }

    let row = sqlx::query_as::<_, RoasteryRow>(
        "SELECT * FROM coffee_roasteries WHERE slug = $1 AND is_published = TRUE",
    )
    .bind(id_or_slug.trim())
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_published(
    pool: &PgPool,
    cluster: Option<&str>,
    query: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<RoasteryRow>> {
    let clean_cluster = cluster
        .map(str::trim)
        .filter(|c| !c.is_empty() && !c.eq_ignore_ascii_case("semua"));
    let search_pattern = query
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));

    let rows = sqlx::query_as::<_, RoasteryRow>(
        r#"SELECT r.*
           FROM coffee_roasteries r
           WHERE r.is_published = TRUE
             AND ($1::text IS NULL OR r.cluster = $1)
             AND (
                 $2::text IS NULL
                 OR r.name ILIKE $2
                 OR r.city ILIKE $2
                 OR r.specialties ILIKE $2
                 OR EXISTS (
                     SELECT 1 FROM roastery_coffee_lineups l
                     WHERE l.roastery_id = r.id
                       AND (l.name ILIKE $2 OR l.origin ILIKE $2 OR l.variety ILIKE $2 OR l.tasting_notes ILIKE $2)
                 )
             )
           ORDER BY r.total_lineups_count DESC, r.rating DESC, r.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(clean_cluster)
    .bind(search_pattern)
    .bind(limit.clamp(1, 100))
    .bind(offset.max(0))
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
