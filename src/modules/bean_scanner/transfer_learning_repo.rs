use sqlx::PgPool;
use uuid::Uuid;
use crate::core::error::{AppError, AppResult};
use super::transfer_learning_dto::{RegisterTrainedModelRequest, TrainedModelSummaryDto};

pub async fn upsert_trained_model(
    pool: &PgPool,
    req: &RegisterTrainedModelRequest,
) -> AppResult<Uuid> {
    let varietal = req.varietal.clone().unwrap_or_else(|| "Mixed Varieties".to_string());
    let process = req.process.clone().unwrap_or_else(|| "Washed".to_string());
    let tasting_notes = req.tasting_notes.clone().unwrap_or_default();
    let threshold = req.cosine_threshold.unwrap_or(0.72);
    let sample_count = req.augmented_samples_count.unwrap_or(45);
    let val_acc = req.validation_accuracy.unwrap_or(0.98);
    let sub_centroids_json = serde_json::to_value(req.sub_centroids.clone().unwrap_or_default())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let sql = r#"
        INSERT INTO trained_coffee_models (
            class_slug, roastery_name, bean_name, varietal, process,
            tasting_notes, altitude_masl, recommended_recipe_id,
            weights_vector, sub_centroids, cosine_threshold,
            augmented_samples_count, validation_accuracy, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW()
        )
        ON CONFLICT (class_slug) DO UPDATE SET
            roastery_name = EXCLUDED.roastery_name,
            bean_name = EXCLUDED.bean_name,
            varietal = EXCLUDED.varietal,
            process = EXCLUDED.process,
            tasting_notes = EXCLUDED.tasting_notes,
            altitude_masl = EXCLUDED.altitude_masl,
            recommended_recipe_id = EXCLUDED.recommended_recipe_id,
            weights_vector = EXCLUDED.weights_vector,
            sub_centroids = EXCLUDED.sub_centroids,
            cosine_threshold = EXCLUDED.cosine_threshold,
            augmented_samples_count = EXCLUDED.augmented_samples_count,
            validation_accuracy = EXCLUDED.validation_accuracy,
            updated_at = NOW()
        RETURNING id
    "#;

    let id: Uuid = sqlx::query_scalar(sql)
        .bind(&req.class_slug)
        .bind(&req.roastery_name)
        .bind(&req.bean_name)
        .bind(&varietal)
        .bind(&process)
        .bind(&tasting_notes)
        .bind(&req.altitude_masl)
        .bind(&req.recommended_recipe_id)
        .bind(&req.weights_vector)
        .bind(&sub_centroids_json)
        .bind(threshold)
        .bind(sample_count)
        .bind(val_acc)
        .fetch_one(pool)
        .await?;

    Ok(id)
}

pub async fn list_trained_models(pool: &PgPool) -> AppResult<Vec<TrainedModelSummaryDto>> {
    let sql = r#"
        SELECT 
            id, class_slug, roastery_name, bean_name, varietal, process,
            tasting_notes, altitude_masl, recommended_recipe_id,
            cosine_threshold, augmented_samples_count, validation_accuracy,
            created_at
        FROM trained_coffee_models
        ORDER BY created_at DESC
        LIMIT 100
    "#;

    let rows = sqlx::query_as::<_, TrainedModelSummaryDto>(sql)
        .fetch_all(pool)
        .await?;

    Ok(rows)
}
