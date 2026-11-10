use sqlx::PgPool;
use crate::core::error::AppResult;
use crate::modules::vision_dataset::dto::SyncVisionClassRequest;
use crate::modules::vision_dataset::models::VisionClassEntity;

pub async fn list_vision_classes(pool: &PgPool) -> AppResult<Vec<VisionClassEntity>> {
    let records = sqlx::query_as::<_, VisionClassEntity>(
        r#"SELECT id, class_slug, brand, coffee_name, process, tasting_notes, 
                  centroid_vector, verified, created_at, updated_at 
           FROM vision_coffee_classes 
           ORDER BY class_slug ASC"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(records)
}

pub async fn upsert_class(
    pool: &PgPool,
    req: &SyncVisionClassRequest,
    centroid_bytes: Option<Vec<u8>>,
) -> AppResult<()> {
    let notes_json = serde_json::to_value(&req.tasting_notes).unwrap_or_default();

    sqlx::query(
        r#"INSERT INTO vision_coffee_classes (
            class_slug, brand, coffee_name, process, tasting_notes, centroid_vector, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
        ON CONFLICT (class_slug) DO UPDATE SET
            brand = EXCLUDED.brand,
            coffee_name = EXCLUDED.coffee_name,
            process = EXCLUDED.process,
            tasting_notes = EXCLUDED.tasting_notes,
            centroid_vector = COALESCE(EXCLUDED.centroid_vector, vision_coffee_classes.centroid_vector),
            updated_at = NOW()"#,
    )
    .bind(&req.class_slug)
    .bind(&req.brand)
    .bind(&req.coffee_name)
    .bind(&req.process)
    .bind(notes_json)
    .bind(centroid_bytes)
    .execute(pool)
    .await?;

    Ok(())
}
