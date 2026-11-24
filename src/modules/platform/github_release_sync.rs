use serde::Deserialize;
use sqlx::PgPool;
use std::time::Duration;

use crate::{
    core::error::{AppError, AppResult},
    modules::platform::{
        dto::UpdateAppVersionConfigRequest,
        models::AppVersionConfig,
        version_config_repo,
        version_evaluator::parse_semver,
    },
};

const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/rusdifn08/coffe-labs-mobile/releases/latest";

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    html_url: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

pub async fn sync_latest_github_release(pool: &PgPool) -> AppResult<AppVersionConfig> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {}", e)))?;

    let resp = client
        .get(GITHUB_LATEST_RELEASE_API)
        .header("User-Agent", "CoffeeLabs-VersionGatekeeper/2.5")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Gagal menghubungi GitHub Releases: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::ServiceUnavailable(format!(
            "GitHub Releases merespons status {}.",
            resp.status()
        )));
    }

    let rel: GithubRelease = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Gagal parse payload GitHub Release: {}", e)))?;

    let clean_ver = rel.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    let existing = version_config_repo::fetch_version_config(pool).await;

    let next_build = match (parse_semver(&clean_ver), parse_semver(&existing.latest_base_version)) {
        (Some(new_v), Some(old_v)) if new_v > old_v => existing.latest_build_number + 1,
        _ => existing.latest_build_number.max(9),
    };

    let apk_url = rel
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".apk"))
        .map(|a| a.browser_download_url.clone())
        .or(rel.html_url.clone());

    let ipa_url = rel
        .assets
        .iter()
        .find(|a| a.name.to_lowercase().ends_with(".ipa"))
        .map(|a| a.browser_download_url.clone());

    let short_notes = rel
        .body
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(600).collect::<String>())
        .unwrap_or_else(|| format!("Pembaruan resmi Coffee Labs v{}.", clean_ver));

    let req = UpdateAppVersionConfigRequest {
        min_base_version: None,
        latest_base_version: Some(clean_ver),
        min_build_number: None,
        latest_build_number: Some(next_build),
        latest_patch_number: Some(0),
        is_force_update: None,
        is_patch_required: None,
        release_notes: Some(short_notes),
        download_url: apk_url,
        ios_download_url: ipa_url,
        windows_download_url: rel.html_url,
        checksum_sha256: None,
        content_manifest_version: Some(existing.content_manifest_version + 1),
    };

    version_config_repo::update_version_config(pool, &req).await
}
