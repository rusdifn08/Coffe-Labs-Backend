use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::roasteries::{
    dto::{compute_unit_price_labels, UpsertLineupDto},
    models::RoasteryLineupRow,
};

pub async fn list_for_roastery(pool: &PgPool, roastery_id: Uuid) -> AppResult<Vec<RoasteryLineupRow>> {
    let rows = sqlx::query_as::<_, RoasteryLineupRow>(
        r#"SELECT * FROM roastery_coffee_lineups
           WHERE roastery_id = $1
           ORDER BY display_order ASC, created_at DESC"#,
    )
    .bind(roastery_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_for_roasteries_batch(
    pool: &PgPool,
    roastery_ids: &[Uuid],
) -> AppResult<Vec<RoasteryLineupRow>> {
    if roastery_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, RoasteryLineupRow>(
        r#"SELECT * FROM roastery_coffee_lineups
           WHERE roastery_id = ANY($1)
           ORDER BY roastery_id, display_order ASC, created_at DESC"#,
    )
    .bind(roastery_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_owned_lineup(
    pool: &PgPool,
    lineup_id: Uuid,
    roastery_id: Uuid,
) -> AppResult<Option<RoasteryLineupRow>> {
    let row = sqlx::query_as::<_, RoasteryLineupRow>(
        "SELECT * FROM roastery_coffee_lineups WHERE id = $1 AND roastery_id = $2",
    )
    .bind(lineup_id)
    .bind(roastery_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn insert_lineup(
    pool: &PgPool,
    roastery_id: Uuid,
    dto: &UpsertLineupDto,
) -> AppResult<RoasteryLineupRow> {
    let (price_100g, price_200g, original_pkg) =
        compute_unit_price_labels(dto.net_weight_grams, dto.price_idr);

    let row = sqlx::query_as::<_, RoasteryLineupRow>(
        r#"INSERT INTO roastery_coffee_lineups (
               roastery_id, name, origin, roast_profile, variety, process, tasting_notes,
               net_weight_grams, price_idr, price_100g_label, price_200g_label,
               original_package_label, bag_image_url, bag_image_public_id,
               badge, is_available, display_order,
               recommended_brew_method, recommended_temp_c, recommended_ratio, recommended_grind_size
           ) VALUES (
               $1, $2, $3, $4, $5, $6, $7,
               $8, $9, $10, $11,
               $12, $13, $14,
               $15, $16, $17,
               $18, $19, $20, $21
           )
           RETURNING *"#,
    )
    .bind(roastery_id)
    .bind(dto.name.trim())
    .bind(dto.origin.trim())
    .bind(dto.roast_profile.trim())
    .bind(dto.variety.trim())
    .bind(dto.process.trim())
    .bind(dto.tasting_notes.trim())
    .bind(dto.net_weight_grams)
    .bind(dto.price_idr)
    .bind(price_100g)
    .bind(price_200g)
    .bind(original_pkg)
    .bind(dto.bag_image_url.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.bag_image_public_id.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.badge.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.is_available.unwrap_or(true))
    .bind(dto.display_order.unwrap_or(0))
    .bind(dto.recommended_brew_method.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.recommended_temp_c)
    .bind(dto.recommended_ratio.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.recommended_grind_size.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_lineup(
    pool: &PgPool,
    lineup_id: Uuid,
    roastery_id: Uuid,
    dto: &UpsertLineupDto,
) -> AppResult<RoasteryLineupRow> {
    let (price_100g, price_200g, original_pkg) =
        compute_unit_price_labels(dto.net_weight_grams, dto.price_idr);

    let row = sqlx::query_as::<_, RoasteryLineupRow>(
        r#"UPDATE roastery_coffee_lineups SET
               name = $3,
               origin = $4,
               roast_profile = $5,
               variety = $6,
               process = $7,
               tasting_notes = $8,
               net_weight_grams = $9,
               price_idr = $10,
               price_100g_label = $11,
               price_200g_label = $12,
               original_package_label = $13,
               bag_image_url = COALESCE($14, bag_image_url),
               bag_image_public_id = COALESCE($15, bag_image_public_id),
               badge = $16,
               is_available = COALESCE($17, is_available),
               display_order = COALESCE($18, display_order),
               recommended_brew_method = $19,
               recommended_temp_c = $20,
               recommended_ratio = $21,
               recommended_grind_size = $22,
               updated_at = NOW()
           WHERE id = $1 AND roastery_id = $2
           RETURNING *"#,
    )
    .bind(lineup_id)
    .bind(roastery_id)
    .bind(dto.name.trim())
    .bind(dto.origin.trim())
    .bind(dto.roast_profile.trim())
    .bind(dto.variety.trim())
    .bind(dto.process.trim())
    .bind(dto.tasting_notes.trim())
    .bind(dto.net_weight_grams)
    .bind(dto.price_idr)
    .bind(price_100g)
    .bind(price_200g)
    .bind(original_pkg)
    .bind(dto.bag_image_url.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.bag_image_public_id.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.badge.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.is_available)
    .bind(dto.display_order)
    .bind(dto.recommended_brew_method.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.recommended_temp_c)
    .bind(dto.recommended_ratio.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.recommended_grind_size.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Varian kopi lineup tidak ditemukan atau bukan milik Anda.".to_string()))?;

    Ok(row)
}

pub async fn delete_lineup(
    pool: &PgPool,
    lineup_id: Uuid,
    roastery_id: Uuid,
) -> AppResult<RoasteryLineupRow> {
    let row = sqlx::query_as::<_, RoasteryLineupRow>(
        "DELETE FROM roastery_coffee_lineups WHERE id = $1 AND roastery_id = $2 RETURNING *",
    )
    .bind(lineup_id)
    .bind(roastery_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Varian kopi lineup tidak ditemukan atau bukan milik Anda.".to_string()))?;

    Ok(row)
}
