use sqlx::PgPool;
use crate::core::error::AppResult;
use crate::modules::vision_dataset::dto::{
    SyncVisionClassRequest, SyncVisionClassResponse, VisionClassItemDto,
};
use crate::modules::vision_dataset::repository;

pub async fn get_vision_classes(pool: &PgPool) -> AppResult<Vec<VisionClassItemDto>> {
    let entities = repository::list_vision_classes(pool).await?;

    let items = entities
        .into_iter()
        .map(|e| {
            let notes: Vec<String> = serde_json::from_value(e.tasting_notes).unwrap_or_default();
            let vector = e.centroid_vector.map(|b| bytes_to_f32(&b));

            VisionClassItemDto {
                class_slug: e.class_slug,
                brand: e.brand,
                coffee_name: e.coffee_name,
                process: e.process,
                tasting_notes: notes,
                centroid_vector: vector,
                verified: e.verified,
                created_at: e.created_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(items)
}

pub async fn sync_vision_class(
    pool: &PgPool,
    req: SyncVisionClassRequest,
) -> AppResult<SyncVisionClassResponse> {
    let bytes = req.centroid_vector.as_ref().map(|v| f32_to_bytes(v));
    let slug = req.class_slug.clone();

    repository::upsert_class(pool, &req, bytes).await?;

    Ok(SyncVisionClassResponse {
        success: true,
        message: format!("Class '{}' berhasil disinkronkan ke katalog cloud.", slug),
        class_slug: slug,
    })
}

fn f32_to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vec.len() * 4);
    for &f in vec {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    let mut vec = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
        vec.push(f32::from_le_bytes(arr));
    }
    vec
}
