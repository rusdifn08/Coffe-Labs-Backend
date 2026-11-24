use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CloudDatasetSummary {
    pub dataset_key: String,
    pub version: i32,
    pub sha256: String,
    pub content_type: String,
    pub object_storage_url: Option<String>,
    pub byte_size: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CloudDatasetRecord {
    pub dataset_key: String,
    pub version: i32,
    pub sha256: String,
    pub content_type: String,
    pub payload_json: Value,
    pub object_storage_url: Option<String>,
    pub byte_size: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertDatasetRequest {
    pub payload_json: Value,
    pub object_storage_url: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CloudMediaAssetRecord {
    pub asset_path: String,
    pub object_url: String,
    pub fallback_url: Option<String>,
    pub storage_provider: String,
    pub content_type: String,
    pub sha256: Option<String>,
    pub compressed_bytes: i64,
    pub original_bytes: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertMediaAssetItem {
    pub asset_path: String,
    pub object_url: String,
    pub fallback_url: Option<String>,
    pub storage_provider: Option<String>,
    pub content_type: Option<String>,
    pub sha256: Option<String>,
    pub compressed_bytes: Option<i64>,
    pub original_bytes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertMediaBatchRequest {
    pub items: Vec<UpsertMediaAssetItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageEndpointsResponse {
    pub provider: String,
    pub asset_cdn_base_url: String,
    pub asset_cdn_fallback_urls: Vec<String>,
    pub cloudinary_configured: bool,
    pub cloudinary_cloud_name: Option<String>,
    pub dataset_api_prefix: String,
}
