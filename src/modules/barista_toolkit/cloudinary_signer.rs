use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use secrecy::ExposeSecret;
use sha2::{Digest, Sha256};

use crate::core::config::CloudinaryConfig;

/// Mengurai `CLOUDINARY_URL` aman dan membentuk Signed URL v3 (`s--<sig8>--/exp_<ts>`).
pub fn generate_signed_raw_url(
    creds: &CloudinaryConfig,
    public_id: &str,
    expires_in_sec: u64,
) -> String {
    let clean_pid = public_id.trim().trim_start_matches('/');
    let exp_ts = Utc::now().timestamp() + expires_in_sec.max(60) as i64;
    let payload = format!("exp={exp_ts}/{clean_pid}{}", creds.api_secret.expose_secret());
    let hash = Sha256::digest(payload.as_bytes());
    let b64 = URL_SAFE_NO_PAD.encode(hash);
    let sig8 = &b64[..b64.len().min(8)];

    format!(
        "https://res.cloudinary.com/{}/raw/authenticated/s--{sig8}--/exp_{exp_ts}/{clean_pid}",
        creds.cloud_name
    )
}

/// Menghasilkan URL gambar teroptimasi otomatis Edge (`f_auto,q_auto:best,dpr_auto,w_{width}`).
pub fn generate_optimized_image_url(
    creds: Option<&CloudinaryConfig>,
    public_id: &str,
    width: u32,
) -> String {
    let clean_pid = public_id.trim().trim_start_matches('/');
    match creds {
        Some(c) => format!(
            "https://res.cloudinary.com/{}/image/upload/f_auto,q_auto:best,dpr_auto,w_{}/{clean_pid}",
            c.cloud_name,
            width.clamp(120, 2400)
        ),
        None => format!("/api/v1/toolkit/object-storage/{clean_pid}"),
    }
}

/// Menghasilkan URL Adaptive HLS Video Stream (`sp_auto/{public_id}.m3u8`).
pub fn generate_hls_stream_url(
    creds: Option<&CloudinaryConfig>,
    public_id: &str,
) -> String {
    let clean_pid = public_id.trim().trim_start_matches('/');
    let trimmed_ext = clean_pid.strip_suffix(".m3u8").unwrap_or(clean_pid);
    match creds {
        Some(c) => format!(
            "https://res.cloudinary.com/{}/video/upload/sp_auto/{trimmed_ext}.m3u8",
            c.cloud_name
        ),
        None => format!("/api/v1/toolkit/object-storage/{trimmed_ext}.m3u8"),
    }
}

/// Memilih URL terbaik untuk aset E-Book biner (.pdf/.epub/.clbook): Cloudinary Authenticated bila tersedia.
pub fn resolve_protected_book_asset_url(
    creds: Option<&CloudinaryConfig>,
    cloudinary_public_id: Option<&str>,
    fallback_backend_path: String,
    expires_in_sec: u64,
) -> String {
    if let (Some(c), Some(pid)) = (creds, cloudinary_public_id.map(str::trim).filter(|s| !s.is_empty())) {
        return generate_signed_raw_url(c, pid, expires_in_sec);
    }
    fallback_backend_path
}
