use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::radar::{
        dto::{validate_indonesia_coords, BulkImportPlaceItem},
        models::{BulkImportResultDto, RadarPlaceKind},
    },
};

pub async fn bulk_upsert_places(
    pool: &PgPool,
    items: &[BulkImportPlaceItem],
) -> AppResult<BulkImportResultDto> {
    let mut ids = Vec::with_capacity(items.len());
    let mut ext_ids = Vec::with_capacity(items.len());
    let mut names = Vec::with_capacity(items.len());
    let mut kinds = Vec::with_capacity(items.len());
    let mut addrs = Vec::with_capacity(items.len());
    let mut cities = Vec::with_capacity(items.len());
    let mut provs = Vec::with_capacity(items.len());
    let mut lats = Vec::with_capacity(items.len());
    let mut lngs = Vec::with_capacity(items.len());
    let mut ratings = Vec::with_capacity(items.len());
    let mut beans = Vec::with_capacity(items.len());
    let mut methods = Vec::with_capacity(items.len());
    let mut skipped = 0usize;

    for (idx, it) in items.iter().enumerate() {
        if it.name.trim().is_empty() || !validate_indonesia_coords(it.lat, it.lng) {
            skipped += 1;
            continue;
        }
        let k = RadarPlaceKind::parse_opt(Some(&it.kind)).unwrap_or(RadarPlaceKind::Cafe);
        ids.push(Uuid::now_v7());
        ext_ids.push(it.external_place_id.clone().unwrap_or_else(|| {
            format!("bulk_{:.5}_{:.5}_{idx}", it.lat, it.lng)
        }));
        names.push(it.name.trim().to_string());
        kinds.push(k.as_db_str().to_string());
        addrs.push(it.address.trim().to_string());
        cities.push(it.city.trim().to_string());
        provs.push(it.province.clone().unwrap_or_else(|| "Indonesia".into()));
        lats.push(it.lat);
        lngs.push(it.lng);
        ratings.push(it.rating.unwrap_or(4.5).clamp(0.0, 5.0));
        beans.push(it.featured_beans.clone().unwrap_or_else(|| Value::Array(vec![])));
        methods.push(it.brewing_methods.clone().unwrap_or_else(|| json!(["V60", "Espresso"])));
    }

    if ids.is_empty() {
        return Ok(BulkImportResultDto {
            upserted_count: 0,
            skipped_invalid_count: skipped,
        });
    }

    let upserted = sqlx::query(
        r#"
        INSERT INTO radar_places (
            id, external_place_id, name, kind, address, city, province,
            latitude, longitude, location, rating, featured_beans, brewing_methods,
            is_active, updated_at
        )
        SELECT
            u.id, u.ext_id, u.name, u.kind::radar_place_kind, u.addr, u.city, u.prov,
            u.lat, u.lng, ST_SetSRID(ST_MakePoint(u.lng, u.lat), 4326)::geography,
            u.rating, u.beans, u.methods, TRUE, NOW()
        FROM UNNEST(
            $1::uuid[], $2::text[], $3::text[], $4::text[], $5::text[],
            $6::text[], $7::text[], $8::float8[], $9::float8[], $10::real[],
            $11::jsonb[], $12::jsonb[]
        ) AS u(id, ext_id, name, kind, addr, city, prov, lat, lng, rating, beans, methods)
        ON CONFLICT (external_place_id) DO UPDATE SET
            name = EXCLUDED.name,
            kind = EXCLUDED.kind,
            address = EXCLUDED.address,
            city = EXCLUDED.city,
            latitude = EXCLUDED.latitude,
            longitude = EXCLUDED.longitude,
            location = EXCLUDED.location,
            featured_beans = EXCLUDED.featured_beans,
            updated_at = NOW()
        "#,
    )
    .bind(&ids)
    .bind(&ext_ids)
    .bind(&names)
    .bind(&kinds)
    .bind(&addrs)
    .bind(&cities)
    .bind(&provs)
    .bind(&lats)
    .bind(&lngs)
    .bind(&ratings)
    .bind(&beans)
    .bind(&methods)
    .execute(pool)
    .await?
    .rows_affected() as usize;

    Ok(BulkImportResultDto {
        upserted_count: upserted,
        skipped_invalid_count: skipped,
    })
}
