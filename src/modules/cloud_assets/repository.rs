use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::{
    core::error::{AppError, AppResult},
    modules::cloud_assets::models::{
        CloudDatasetRecord, CloudDatasetSummary, CloudMediaAssetRecord, UpsertDatasetRequest,
        UpsertMediaAssetItem,
    },
};

pub fn validate_dataset_key(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().to_lowercase();
    let valid = !trimmed.is_empty()
        && trimmed.len() <= 120
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if !valid || trimmed.contains("..") {
        return Err(AppError::Validation(
            "dataset_key hanya boleh berisi [a-z0-9_-.] tanpa '..'".to_string(),
        ));
    }
    Ok(trimmed)
}

pub fn validate_asset_path(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim().trim_start_matches('/');
    let valid = !trimmed.is_empty()
        && trimmed.len() <= 260
        && !trimmed.contains("..")
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'));
    if !valid {
        return Err(AppError::Validation(
            "asset_path tidak valid untuk registri object storage.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

pub async fn list_dataset_summaries(pool: &PgPool) -> AppResult<Vec<CloudDatasetSummary>> {
    let rows = sqlx::query_as::<_, CloudDatasetSummary>(
        r#"SELECT dataset_key, version, sha256, content_type, object_storage_url, byte_size, updated_at
           FROM app_cloud_datasets
           ORDER BY dataset_key ASC"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_dataset_by_key(
    pool: &PgPool,
    dataset_key: &str,
) -> AppResult<Option<CloudDatasetRecord>> {
    let row = sqlx::query_as::<_, CloudDatasetRecord>(
        r#"SELECT dataset_key, version, sha256, content_type, payload_json, object_storage_url, byte_size, updated_at
           FROM app_cloud_datasets
           WHERE dataset_key = $1"#,
    )
    .bind(dataset_key)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn upsert_dataset(
    pool: &PgPool,
    dataset_key: &str,
    req: &UpsertDatasetRequest,
) -> AppResult<CloudDatasetSummary> {
    let serialized = serde_json::to_vec(&req.payload_json)
        .map_err(|e| AppError::BadRequest(format!("Gagal serialisasi payload JSON: {e}")))?;
    let sha256 = hex::encode(Sha256::digest(&serialized));
    let byte_size = serialized.len() as i64;
    let content_type = req
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("application/json");

    let row = sqlx::query_as::<_, CloudDatasetSummary>(
        r#"INSERT INTO app_cloud_datasets (
               dataset_key, version, sha256, content_type, payload_json, object_storage_url, byte_size, updated_at
           )
           VALUES ($1, 1, $2, $3, $4, $5, $6, NOW())
           ON CONFLICT (dataset_key) DO UPDATE SET
               version = CASE
                   WHEN app_cloud_datasets.sha256 = EXCLUDED.sha256 THEN app_cloud_datasets.version
                   ELSE app_cloud_datasets.version + 1
               END,
               sha256 = EXCLUDED.sha256,
               content_type = EXCLUDED.content_type,
               payload_json = EXCLUDED.payload_json,
               object_storage_url = COALESCE(EXCLUDED.object_storage_url, app_cloud_datasets.object_storage_url),
               byte_size = EXCLUDED.byte_size,
               updated_at = NOW()
           RETURNING dataset_key, version, sha256, content_type, object_storage_url, byte_size, updated_at"#,
    )
    .bind(dataset_key)
    .bind(&sha256)
    .bind(content_type)
    .bind(&req.payload_json)
    .bind(req.object_storage_url.as_deref())
    .bind(byte_size)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn list_media_assets(
    pool: &PgPool,
    prefix: Option<&str>,
) -> AppResult<Vec<CloudMediaAssetRecord>> {
    let rows = if let Some(p) = prefix.map(str::trim).filter(|s| !s.is_empty()) {
        let like_pattern = format!("{p}%");
        sqlx::query_as::<_, CloudMediaAssetRecord>(
            r#"SELECT asset_path, object_url, fallback_url, storage_provider, content_type,
                      sha256, compressed_bytes, original_bytes, updated_at
               FROM app_cloud_media_assets
               WHERE asset_path LIKE $1
               ORDER BY asset_path ASC
               LIMIT 1000"#,
        )
        .bind(like_pattern)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, CloudMediaAssetRecord>(
            r#"SELECT asset_path, object_url, fallback_url, storage_provider, content_type,
                      sha256, compressed_bytes, original_bytes, updated_at
               FROM app_cloud_media_assets
               ORDER BY asset_path ASC
               LIMIT 1000"#,
        )
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

pub async fn upsert_media_batch(
    pool: &PgPool,
    items: &[UpsertMediaAssetItem],
) -> AppResult<usize> {
    let mut updated = 0usize;
    for item in items {
        let clean_path = validate_asset_path(&item.asset_path)?;
        let object_url = item.object_url.trim();
        if !object_url.starts_with("https://") {
            return Err(AppError::Validation(format!(
                "object_url untuk '{clean_path}' harus menggunakan HTTPS."
            )));
        }
        let provider = item
            .storage_provider
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("MULTI_CDN");
        let content_type = item
            .content_type
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("image/jpeg");

        sqlx::query(
            r#"INSERT INTO app_cloud_media_assets (
                   asset_path, object_url, fallback_url, storage_provider, content_type,
                   sha256, compressed_bytes, original_bytes, updated_at
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
               ON CONFLICT (asset_path) DO UPDATE SET
                   object_url = EXCLUDED.object_url,
                   fallback_url = COALESCE(EXCLUDED.fallback_url, app_cloud_media_assets.fallback_url),
                   storage_provider = EXCLUDED.storage_provider,
                   content_type = EXCLUDED.content_type,
                   sha256 = COALESCE(EXCLUDED.sha256, app_cloud_media_assets.sha256),
                   compressed_bytes = EXCLUDED.compressed_bytes,
                   original_bytes = EXCLUDED.original_bytes,
                   updated_at = NOW()"#,
        )
        .bind(&clean_path)
        .bind(object_url)
        .bind(item.fallback_url.as_deref())
        .bind(provider)
        .bind(content_type)
        .bind(item.sha256.as_deref())
        .bind(item.compressed_bytes.unwrap_or(0))
        .bind(item.original_bytes.unwrap_or(0))
        .execute(pool)
        .await?;
        updated += 1;
    }
    Ok(updated)
}
