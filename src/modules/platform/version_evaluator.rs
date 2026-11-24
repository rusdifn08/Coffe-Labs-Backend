use std::cmp::Ordering;

use crate::modules::platform::{
    dto::{VersionCheckQuery, VersionCheckResponse},
    models::{AppVersionConfig, UpdateAction},
};

const DEFAULT_RELEASES_URL: &str = "https://github.com/rusdifn08/coffe-labs-mobile/releases/latest";

/// Parse SemVer string such as "2.5.1", "v2.5.1", or "2.5.1+9" into (major, minor, patch).
pub fn parse_semver(raw: &str) -> Option<(u32, u32, u32)> {
    let clean = raw
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .split('+')
        .next()?
        .split('-')
        .next()?;
    let mut parts = clean.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    let patch = parts.next().unwrap_or("0").parse::<u32>().ok()?;
    Some((major, minor, patch))
}

/// Normalizes Flutter Android `--split-per-abi` versionCode offsets (e.g. 2009 -> 9, 1008 -> 8).
pub fn normalize_abi_build_number(raw: i32) -> i32 {
    if (1000..10000).contains(&raw) {
        let remainder = raw % 1000;
        if remainder > 0 {
            return remainder;
        }
    }
    raw.max(1)
}

fn compare_client_to_target(
    client_semver: Option<(u32, u32, u32)>,
    client_build: i32,
    target_semver: Option<(u32, u32, u32)>,
    target_build: i32,
) -> Ordering {
    if let (Some(c_ver), Some(t_ver)) = (client_semver, target_semver) {
        let ord = c_ver.cmp(&t_ver);
        if ord != Ordering::Equal {
            return ord;
        }
    }
    client_build.cmp(&normalize_abi_build_number(target_build))
}

pub fn resolve_platform_download_url(config: &AppVersionConfig, platform: Option<&str>) -> Option<String> {
    let plat = platform.unwrap_or("android").trim().to_lowercase();
    let candidate = match plat.as_str() {
        "ios" => config
            .ios_download_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(config.download_url.as_deref()),
        "windows" | "macos" | "linux" | "web" => config
            .windows_download_url
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(Some(DEFAULT_RELEASES_URL)),
        _ => config.download_url.as_deref(),
    };
    let resolved = candidate
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.contains("rusdi-labs/coffee-hub-mobile"))
        .unwrap_or(DEFAULT_RELEASES_URL);
    Some(resolved.to_string())
}

pub fn evaluate_version_check(
    config: &AppVersionConfig,
    params: &VersionCheckQuery,
) -> VersionCheckResponse {
    let client_build = normalize_abi_build_number(params.build_number.unwrap_or(1));
    let client_patch = params.patch_number.unwrap_or(0).max(0);
    let client_ver = params.base_version.as_deref().and_then(parse_semver);
    let min_ver = parse_semver(&config.min_base_version);
    let latest_ver = parse_semver(&config.latest_base_version);

    let cmp_min = compare_client_to_target(client_ver, client_build, min_ver, config.min_build_number);
    let cmp_latest = compare_client_to_target(client_ver, client_build, latest_ver, config.latest_build_number);

    let (action, effective_force) = if cmp_min == Ordering::Less {
        (UpdateAction::ForceStoreUpdate, true)
    } else if cmp_latest == Ordering::Less {
        if config.is_force_update {
            (UpdateAction::ForceStoreUpdate, true)
        } else {
            (UpdateAction::FlexibleStoreUpdate, false)
        }
    } else if cmp_latest == Ordering::Equal && client_patch < config.latest_patch_number {
        if config.is_patch_required {
            (UpdateAction::ForceOtaPatch, true)
        } else {
            (UpdateAction::SilentOtaPatch, false)
        }
    } else {
        (UpdateAction::None, false)
    };

    let (action_code, update_channel) = match action {
        UpdateAction::None => ("UP_TO_DATE", "up_to_date"),
        UpdateAction::SilentOtaPatch => ("SILENT_OTA_PATCH", "shorebird_ota"),
        UpdateAction::ForceOtaPatch => ("FORCE_OTA_PATCH", "shorebird_ota"),
        UpdateAction::FlexibleStoreUpdate => ("FLEXIBLE_STORE_UPDATE", "native_browser"),
        UpdateAction::ForceStoreUpdate => ("FORCE_STORE_UPDATE", "native_browser"),
    };

    let target_patch = (config.latest_patch_number > 0).then_some(config.latest_patch_number);

    VersionCheckResponse {
        action,
        action_code: action_code.to_string(),
        update_channel: update_channel.to_string(),
        target_patch,
        latest_base_version: config.latest_base_version.clone(),
        latest_build_number: normalize_abi_build_number(config.latest_build_number),
        normalized_client_build: client_build,
        is_force_update: effective_force,
        release_notes: config.release_notes.clone(),
        download_url: resolve_platform_download_url(config, params.platform.as_deref()),
        checksum_sha256: config.checksum_sha256.clone(),
        content_manifest_version: config.content_manifest_version.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm64_split_abi_older_version_gets_update() {
        let cfg = AppVersionConfig::fallback_default(); // v2.5.1 build 9
        let q = VersionCheckQuery {
            platform: Some("android".into()),
            base_version: Some("2.5.0".into()),
            build_number: Some(2008),
            patch_number: Some(0),
        };
        let res = evaluate_version_check(&cfg, &q);
        assert_eq!(res.action_code, "FLEXIBLE_STORE_UPDATE");
        assert_eq!(res.normalized_client_build, 8);
        assert!(!res.is_force_update);
        assert!(res.download_url.as_deref().unwrap_or("").ends_with("Coffe_Labs_25.apk"));
    }

    #[test]
    fn latest_version_is_never_locked_even_when_force_update_enabled() {
        let mut cfg = AppVersionConfig::fallback_default();
        cfg.is_force_update = true;
        let q = VersionCheckQuery {
            platform: Some("android".into()),
            base_version: Some("2.5.1".into()),
            build_number: Some(2009),
            patch_number: Some(0),
        };
        let res = evaluate_version_check(&cfg, &q);
        assert_eq!(res.action_code, "UP_TO_DATE");
        assert!(!res.is_force_update);
    }

    #[test]
    fn shorebird_delta_patch_triggers_on_matching_base_version() {
        let mut cfg = AppVersionConfig::fallback_default();
        cfg.latest_patch_number = 2;
        cfg.is_patch_required = false;
        let q = VersionCheckQuery {
            platform: Some("android".into()),
            base_version: Some("2.5.1".into()),
            build_number: Some(2009),
            patch_number: Some(1),
        };
        let res = evaluate_version_check(&cfg, &q);
        assert_eq!(res.action_code, "SILENT_OTA_PATCH");
        assert_eq!(res.update_channel, "shorebird_ota");
        assert_eq!(res.target_patch, Some(2));
    }

    #[test]
    fn ios_receives_ipa_download_url() {
        let cfg = AppVersionConfig::fallback_default();
        let q = VersionCheckQuery {
            platform: Some("ios".into()),
            base_version: Some("2.4.0".into()),
            build_number: Some(7),
            patch_number: Some(0),
        };
        let res = evaluate_version_check(&cfg, &q);
        assert_eq!(res.action_code, "FLEXIBLE_STORE_UPDATE");
        assert!(res.download_url.as_deref().unwrap_or("").ends_with("CoffeLabs25.ipa"));
    }
}
