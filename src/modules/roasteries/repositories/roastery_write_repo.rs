use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::roasteries::{
    dto::{CreateRoasteryDto, UpdateRoasteryDto},
    models::RoasteryRow,
};

pub async fn insert_roastery(
    pool: &PgPool,
    owner_user_id: Uuid,
    slug: &str,
    dto: &CreateRoasteryDto,
) -> AppResult<RoasteryRow> {
    let cluster = dto
        .cluster
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Roastery Bisnis");
    let highlight = dto
        .sales_highlight
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Roastery Bisnis Terverifikasi");

    let res = sqlx::query_as::<_, RoasteryRow>(
        r#"INSERT INTO coffee_roasteries (
               owner_user_id, slug, name, city, address, latitude, longitude,
               cluster, description, specialties, sales_highlight,
               logo_url, logo_public_id, banner_url, banner_public_id,
               whatsapp_number, instagram_handle, website_url
           ) VALUES (
               $1, $2, $3, $4, $5, $6, $7,
               $8, $9, $10, $11,
               $12, $13, $14, $15,
               $16, $17, $18
           )
           RETURNING *"#,
    )
    .bind(owner_user_id)
    .bind(slug)
    .bind(dto.name.trim())
    .bind(dto.city.trim())
    .bind(dto.address.trim())
    .bind(dto.latitude.unwrap_or(-6.2088))
    .bind(dto.longitude.unwrap_or(106.8456))
    .bind(cluster)
    .bind(dto.description.trim())
    .bind(dto.specialties.as_deref().unwrap_or("").trim())
    .bind(highlight)
    .bind(dto.logo_url.trim())
    .bind(dto.logo_public_id.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.banner_url.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.banner_public_id.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.whatsapp_number.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.instagram_handle.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(dto.website_url.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .fetch_one(pool)
    .await;

    match res {
        Ok(row) => Ok(row),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
            Err(AppError::Conflict(
                "Akun Business Anda sudah memiliki 1 Roastery terdaftar. Gunakan halaman Kelola Roastery Saya."
                    .to_string(),
            ))
        }
        Err(err) => Err(AppError::Database(err)),
    }
}

pub async fn update_roastery_by_owner(
    pool: &PgPool,
    owner_user_id: Uuid,
    dto: &UpdateRoasteryDto,
) -> AppResult<RoasteryRow> {
    let row = sqlx::query_as::<_, RoasteryRow>(
        r#"UPDATE coffee_roasteries SET
               name = COALESCE($2, name),
               city = COALESCE($3, city),
               address = COALESCE($4, address),
               latitude = COALESCE($5, latitude),
               longitude = COALESCE($6, longitude),
               cluster = COALESCE($7, cluster),
               description = COALESCE($8, description),
               specialties = COALESCE($9, specialties),
               sales_highlight = COALESCE($10, sales_highlight),
               logo_url = COALESCE($11, logo_url),
               logo_public_id = COALESCE($12, logo_public_id),
               banner_url = COALESCE($13, banner_url),
               banner_public_id = COALESCE($14, banner_public_id),
               whatsapp_number = COALESCE($15, whatsapp_number),
               instagram_handle = COALESCE($16, instagram_handle),
               website_url = COALESCE($17, website_url),
               is_published = COALESCE($18, is_published),
               updated_at = NOW()
           WHERE owner_user_id = $1
           RETURNING *"#,
    )
    .bind(owner_user_id)
    .bind(dto.name.as_deref().map(str::trim))
    .bind(dto.city.as_deref().map(str::trim))
    .bind(dto.address.as_deref().map(str::trim))
    .bind(dto.latitude)
    .bind(dto.longitude)
    .bind(dto.cluster.as_deref().map(str::trim))
    .bind(dto.description.as_deref().map(str::trim))
    .bind(dto.specialties.as_deref().map(str::trim))
    .bind(dto.sales_highlight.as_deref().map(str::trim))
    .bind(dto.logo_url.as_deref().map(str::trim))
    .bind(dto.logo_public_id.as_deref().map(str::trim))
    .bind(dto.banner_url.as_deref().map(str::trim))
    .bind(dto.banner_public_id.as_deref().map(str::trim))
    .bind(dto.whatsapp_number.as_deref().map(str::trim))
    .bind(dto.instagram_handle.as_deref().map(str::trim))
    .bind(dto.website_url.as_deref().map(str::trim))
    .bind(dto.is_published)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Profil Roastery Anda belum dibuat.".to_string()))?;

    Ok(row)
}

pub async fn sync_lineups_count(pool: &PgPool, roastery_id: Uuid) -> AppResult<i32> {
    let count: i32 = sqlx::query_scalar(
        r#"UPDATE coffee_roasteries r
           SET total_lineups_count = (
               SELECT COUNT(*)::int FROM roastery_coffee_lineups l WHERE l.roastery_id = r.id
           ),
           updated_at = NOW()
           WHERE r.id = $1
           RETURNING total_lineups_count"#,
    )
    .bind(roastery_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}
