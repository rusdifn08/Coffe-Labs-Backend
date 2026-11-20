use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::beans_radar::{
        models::CoffeeRadarPlaceRow,
        repositories::radar_place_repo::batch_hydrate_by_ids,
    },
};

pub async fn is_place_favorited(pool: &PgPool, user_id: Uuid, place_id: Uuid) -> AppResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_favorite_coffee_places WHERE user_id = $1 AND place_id = $2)",
    )
    .bind(user_id)
    .bind(place_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

pub async fn toggle_favorite(pool: &PgPool, user_id: Uuid, place_id: Uuid) -> AppResult<bool> {
    let deleted = sqlx::query(
        "DELETE FROM user_favorite_coffee_places WHERE user_id = $1 AND place_id = $2",
    )
    .bind(user_id)
    .bind(place_id)
    .execute(pool)
    .await?
    .rows_affected();

    if deleted > 0 {
        return Ok(false);
    }

    sqlx::query(
        r#"INSERT INTO user_favorite_coffee_places (user_id, place_id)
           VALUES ($1, $2)
           ON CONFLICT (user_id, place_id) DO NOTHING"#,
    )
    .bind(user_id)
    .bind(place_id)
    .execute(pool)
    .await?;

    Ok(true)
}

pub async fn list_user_favorite_places(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> AppResult<Vec<CoffeeRadarPlaceRow>> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        r#"SELECT place_id
           FROM user_favorite_coffee_places
           WHERE user_id = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    batch_hydrate_by_ids(pool, &ids).await
}

/// Sinkronkan Roastery akun Business aktif ke dalam tabel `coffee_radar_places` secara otomatis.
pub async fn sync_published_business_roasteries(pool: &PgPool) -> AppResult<u64> {
    let res = sqlx::query(
        r#"INSERT INTO coffee_radar_places (
                id, external_source, external_place_id, linked_roastery_id,
                slug, name, category, sub_categories,
                latitude, longitude, location, h3_res7, h3_res9,
                full_address, district_kecamatan, city_kabupaten, province, timezone,
                rating, reviews_count, bayesian_score, price_level,
                opening_hours_json, is_24_hours, sells_roast_beans, has_manual_brew_v60,
                featured_bean_summary, whatsapp_number, instagram_handle, website_url,
                thumbnail_url, cloudinary_public_id, is_verified_partner, is_active
            )
            SELECT
                r.id, 'COFFEELABS_BUSINESS', 'coffeelabs_roastery_' || r.id::text, r.id,
                'partner-' || r.slug, r.name, 'ROASTERY'::coffee_place_category,
                '["Coffee Roastery", "Partner Terverifikasi"]'::jsonb,
                r.latitude, r.longitude, ST_SetSRID(ST_MakePoint(r.longitude, r.latitude), 4326)::geography, 0, 0,
                r.address, '', r.city, 'Indonesia',
                CASE WHEN r.longitude >= 126.5 THEN 'Asia/Jayapura'
                     WHEN r.longitude >= 114.6 THEN 'Asia/Makassar'
                     ELSE 'Asia/Jakarta' END,
                LEAST(5.0, GREATEST(0.0, r.rating::real)), 50, 4.72, 2,
                '{"mon":[["08:00","22:00"]],"tue":[["08:00","22:00"]],"wed":[["08:00","22:00"]],"thu":[["08:00","22:00"]],"fri":[["08:00","22:00"]],"sat":[["08:00","22:00"]],"sun":[["08:00","22:00"]]}'::jsonb,
                FALSE, TRUE, TRUE, r.sales_highlight, r.whatsapp_number, r.instagram_handle, r.website_url,
                COALESCE(r.banner_url, r.logo_url), COALESCE(r.banner_public_id, r.logo_public_id), TRUE, TRUE
            FROM coffee_roasteries r
            WHERE r.is_published = TRUE
              AND r.latitude BETWEEN -11.1 AND 6.2
              AND r.longitude BETWEEN 94.9 AND 141.1
              AND LOWER(r.address) NOT LIKE '%jl. roastery no. 75%'
            ON CONFLICT (external_place_id) DO UPDATE SET
                name = EXCLUDED.name,
                latitude = EXCLUDED.latitude,
                longitude = EXCLUDED.longitude,
                location = EXCLUDED.location,
                full_address = EXCLUDED.full_address,
                thumbnail_url = EXCLUDED.thumbnail_url,
                is_verified_partner = TRUE,
                updated_at = NOW()"#,
    )
    .execute(pool)
    .await?;

    Ok(res.rows_affected())
}
