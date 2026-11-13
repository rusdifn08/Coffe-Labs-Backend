use std::{collections::BTreeMap, fmt};

use chrono::Utc;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::core::{
    config::{parse_cloudinary_url, AppConfig, CloudinaryConfig},
    error::{AppError, AppResult},
};

const MAX_PARAM_LEN: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct CloudinaryUploadSignature {
    pub timestamp: i64,
    pub signature: String,
    pub api_key: String,
    pub cloud_name: String,
    pub upload_url: String,
    pub folder: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_id: Option<String>,
}

/// Layanan Direct Signed Upload (client-to-Cloudinary) agar upload gambar hemat RAM 512 MB.
#[derive(Clone)]
pub struct CloudinaryStorage {
    credentials: Option<CloudinaryConfig>,
}

impl fmt::Debug for CloudinaryStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudinaryStorage")
            .field("configured", &self.credentials.is_some())
            .field("cloud_name", &self.cloud_name())
            .finish()
    }
}

fn validate_segment(raw: &str, field: &str) -> AppResult<String> {
    let s = raw.trim();
    if s.is_empty() || s.len() > MAX_PARAM_LEN {
        return Err(AppError::Validation(format!(
            "Parameter Cloudinary '{field}' harus 1-{MAX_PARAM_LEN} karakter."
        )));
    }
    let bad_path = s.starts_with('/') || s.ends_with('/') || s.contains("//") || s.contains("..");
    let valid_chars = s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/'));
    if bad_path || !valid_chars {
        return Err(AppError::Validation(format!(
            "Parameter Cloudinary '{field}' hanya boleh berisi path [a-zA-Z0-9_/-]."
        )));
    }
    Ok(s.to_string())
}

impl CloudinaryStorage {
    pub fn from_config(config: &AppConfig) -> Self {
        match config.cloudinary_config() {
            Ok(Some(creds)) => {
                info!(cloud_name = %creds.cloud_name, "☁️ Cloudinary Direct Signed Upload aktif.");
                Self { credentials: Some(creds) }
            }
            Ok(None) => {
                info!("🧊 CLOUDINARY_URL belum disetel; layanan upload dalam mode standby.");
                Self::unconfigured()
            }
            Err(err) => {
                warn!(error = %err, "⚠️ CLOUDINARY_URL tidak valid; layanan Cloudinary dinonaktifkan.");
                Self::unconfigured()
            }
        }
    }

    pub fn from_url(raw_url: &str) -> AppResult<Self> {
        Ok(Self { credentials: Some(parse_cloudinary_url(raw_url).map_err(AppError::BadRequest)?) })
    }

    pub fn unconfigured() -> Self { Self { credentials: None } }
    pub fn is_configured(&self) -> bool { self.credentials.is_some() }
    pub fn cloud_name(&self) -> Option<&str> { self.credentials.as_ref().map(|c| c.cloud_name.as_str()) }

    pub fn generate_upload_signature(
        &self,
        folder: &str,
        public_id: Option<&str>,
    ) -> AppResult<CloudinaryUploadSignature> {
        self.generate_upload_signature_at(Utc::now().timestamp(), folder, public_id)
    }

    pub fn generate_upload_signature_at(
        &self,
        timestamp: i64,
        folder: &str,
        public_id: Option<&str>,
    ) -> AppResult<CloudinaryUploadSignature> {
        let creds = self.credentials.as_ref().ok_or_else(|| {
            AppError::Internal("CloudinaryStorage belum dikonfigurasi (CLOUDINARY_URL kosong).".into())
        })?;
        if timestamp <= 0 {
            return Err(AppError::Validation("Timestamp upload harus bernilai positif.".into()));
        }

        let clean_folder = validate_segment(folder, "folder")?;
        let clean_public_id = public_id
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| validate_segment(v, "public_id"))
            .transpose()?;

        let ts_str = timestamp.to_string();
        let mut params = BTreeMap::from([("folder", clean_folder.as_str()), ("timestamp", ts_str.as_str())]);
        if let Some(ref pid) = clean_public_id {
            params.insert("public_id", pid.as_str());
        }

        let joined: String = params.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
        let to_sign = format!("{joined}{}", creds.api_secret.expose_secret());
        let signature = hex::encode(Sha1::digest(to_sign.as_bytes()));

        Ok(CloudinaryUploadSignature {
            timestamp,
            signature,
            api_key: creds.api_key.clone(),
            cloud_name: creds.cloud_name.clone(),
            upload_url: format!("https://api.cloudinary.com/v1_1/{}/image/upload", creds.cloud_name),
            folder: clean_folder,
            public_id: clean_public_id,
        })
    }

    pub fn auto_optimize_url(raw_url: &str) -> String { auto_optimize_url(raw_url) }
}

/// Menyisipkan transformasi `f_auto,q_auto` secara idempoten pada URL CDN Cloudinary.
pub fn auto_optimize_url(raw_url: &str) -> String {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let marker = if trimmed.contains("/image/upload/") {
        "/image/upload/"
    } else if trimmed.contains("res.cloudinary.com") && trimmed.contains("/upload/") {
        "/upload/"
    } else {
        return trimmed.to_string();
    };
    let Some((prefix, suffix)) = trimmed.split_once(marker) else {
        return trimmed.to_string();
    };
    let segments: Vec<&str> = suffix.split('?').next().unwrap_or(suffix).split('/').collect();
    let dirs = &segments[..segments.len().saturating_sub(1)];
    let has = |t: &str| dirs.iter().flat_map(|s| s.split(',')).any(|tok| tok == t);
    if has("f_auto") && has("q_auto") {
        return trimmed.to_string();
    }
    format!("{prefix}{marker}f_auto,q_auto/{suffix}")
}
