use sqlx::PgPool;

use crate::{
    core::error::AppResult,
    modules::platform::{dto::UpdateAppVersionConfigRequest, models::AppVersionConfig},
};

pub async fn fetch_version_config(pool: &PgPool) -> AppVersionConfig {
    sqlx::query_as::<_, AppVersionConfig>(
        "SELECT id, platform, min_base_version, latest_base_version,
                min_build_number, latest_build_number, latest_patch_number,
                is_force_update, is_patch_required, release_notes,
                download_url, ios_download_url, windows_download_url,
                checksum_sha256, content_manifest_version, updated_at
         FROM app_version_configs WHERE id = 'default'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| AppVersionConfig::fallback_default())
}

pub async fn update_version_config(
    pool: &PgPool,
    req: &UpdateAppVersionConfigRequest,
) -> AppResult<AppVersionConfig> {
    sqlx::query("INSERT INTO app_version_configs (id) VALUES ('default') ON CONFLICT (id) DO NOTHING")
        .execute(pool)
        .await?;

    let updated = sqlx::query_as::<_, AppVersionConfig>(
        r#"UPDATE app_version_configs
           SET min_base_version = COALESCE($1, min_base_version),
               latest_base_version = COALESCE($2, latest_base_version),
               min_build_number = COALESCE($3, min_build_number),
               latest_build_number = COALESCE($4, latest_build_number),
               latest_patch_number = COALESCE($5, latest_patch_number),
               is_force_update = COALESCE($6, is_force_update),
               is_patch_required = COALESCE($7, is_patch_required),
               release_notes = COALESCE($8, release_notes),
               download_url = COALESCE($9, download_url),
               ios_download_url = COALESCE($10, ios_download_url),
               windows_download_url = COALESCE($11, windows_download_url),
               checksum_sha256 = COALESCE($12, checksum_sha256),
               content_manifest_version = COALESCE($13, content_manifest_version),
               updated_at = NOW()
           WHERE id = 'default'
           RETURNING id, platform, min_base_version, latest_base_version,
                     min_build_number, latest_build_number, latest_patch_number,
                     is_force_update, is_patch_required, release_notes,
                     download_url, ios_download_url, windows_download_url,
                     checksum_sha256, content_manifest_version, updated_at"#,
    )
    .bind(&req.min_base_version)
    .bind(&req.latest_base_version)
    .bind(req.min_build_number)
    .bind(req.latest_build_number)
    .bind(req.latest_patch_number)
    .bind(req.is_force_update)
    .bind(req.is_patch_required)
    .bind(&req.release_notes)
    .bind(&req.download_url)
    .bind(&req.ios_download_url)
    .bind(&req.windows_download_url)
    .bind(&req.checksum_sha256)
    .bind(req.content_manifest_version)
    .fetch_one(pool)
    .await?;
    Ok(updated)
}
