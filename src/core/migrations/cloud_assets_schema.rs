pub fn statements() -> &'static [&'static str] {
    &[
        r#"CREATE TABLE IF NOT EXISTS app_cloud_datasets (
            dataset_key VARCHAR(120) PRIMARY KEY,
            version INTEGER NOT NULL DEFAULT 1,
            sha256 VARCHAR(64) NOT NULL,
            content_type VARCHAR(80) NOT NULL DEFAULT 'application/json',
            payload_json JSONB NOT NULL,
            object_storage_url TEXT,
            byte_size BIGINT NOT NULL DEFAULT 0,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE INDEX IF NOT EXISTS idx_app_cloud_datasets_updated_at
           ON app_cloud_datasets (updated_at DESC)"#,
        r#"CREATE TABLE IF NOT EXISTS app_cloud_media_assets (
            asset_path VARCHAR(260) PRIMARY KEY,
            object_url TEXT NOT NULL,
            fallback_url TEXT,
            storage_provider VARCHAR(40) NOT NULL DEFAULT 'MULTI_CDN',
            content_type VARCHAR(80) NOT NULL DEFAULT 'image/jpeg',
            sha256 VARCHAR(64),
            compressed_bytes BIGINT NOT NULL DEFAULT 0,
            original_bytes BIGINT NOT NULL DEFAULT 0,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE INDEX IF NOT EXISTS idx_app_cloud_media_provider
           ON app_cloud_media_assets (storage_provider)"#,
    ]
}
