use std::io::SeekFrom;
use std::path::PathBuf;

use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, Response, StatusCode},
};
use bytes::Bytes;
use futures_util::stream;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::{
    core::error::{AppError, AppResult},
    modules::barista_toolkit::error::BaristaToolkitError,
};

const CHUNK_SIZE: usize = 64 * 1024;

pub fn validate_object_key(raw_key: &str) -> AppResult<String> {
    let trimmed = raw_key.trim().trim_start_matches('/');
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('\\')
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.'))
    {
        return Err(AppError::BadRequest("Object Storage key tidak valid.".into()));
    }
    Ok(trimmed.to_string())
}

pub fn mime_for_key(key: &str) -> &'static str {
    let lower = key.to_ascii_lowercase();
    if lower.ends_with(".m3u8") {
        "application/vnd.apple.mpegurl"
    } else if lower.ends_with(".ts") {
        "video/mp2t"
    } else if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".vtt") {
        "text/vtt; charset=utf-8"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".epub") {
        "application/epub+zip"
    } else if lower.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

/// Parses standard HTTP `Range: bytes=start-end` header safely without panics.
pub fn parse_byte_range(range_hdr: &str, total_len: u64) -> Option<(u64, u64)> {
    if total_len == 0 {
        return None;
    }
    let spec = range_hdr.trim().strip_prefix("bytes=")?;
    let first_range = spec.split(',').next()?.trim();
    let (start_s, end_s) = first_range.split_once('-')?;

    if start_s.is_empty() {
        let suffix: u64 = end_s.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        let start = total_len.saturating_sub(suffix);
        return Some((start, total_len - 1));
    }

    let start: u64 = start_s.parse().ok()?;
    if start >= total_len {
        return None;
    }
    let end = if end_s.is_empty() {
        total_len - 1
    } else {
        end_s.parse::<u64>().ok()?.min(total_len - 1)
    };
    if end < start {
        return None;
    }
    Some((start, end))
}

/// Checks if a rendered HLS manifest (`.m3u8`) or MP4 exists in Managed Object Storage.
pub async fn resolve_local_lms_stream_path(manifest_path: &str) -> Option<String> {
    let clean = validate_object_key(manifest_path).ok()?;
    let primary = PathBuf::from("storage/object_storage").join(&clean);
    if tokio::fs::metadata(&primary).await.is_ok() {
        return Some(format!("/api/v1/toolkit/object-storage/{clean}"));
    }
    if let Some(stem) = clean.strip_suffix(".m3u8") {
        let mp4_rel = format!("{stem}.mp4");
        let mp4_path = PathBuf::from("storage/object_storage").join(&mp4_rel);
        if tokio::fs::metadata(&mp4_path).await.is_ok() {
            return Some(format!("/api/v1/toolkit/object-storage/{mp4_rel}"));
        }
    }
    None
}

/// Streams Object Storage files (HLS .m3u8/.ts, MP4, WebVTT, Images, E-Books) with
/// full HTTP 206 Range seeking and bounded 64 KB chunk streaming for 512 MB RAM safety.
pub async fn stream_object_file(headers: &HeaderMap, raw_key: &str) -> AppResult<Response<Body>> {
    let clean = validate_object_key(raw_key)?;
    let full_path = PathBuf::from("storage/object_storage").join(&clean);
    let mut file = tokio::fs::File::open(&full_path)
        .await
        .map_err(|_| AppError::from(BaristaToolkitError::NotFound))?;
    let total_len = file
        .metadata()
        .await
        .map_err(|_| AppError::from(BaristaToolkitError::NotFound))?
        .len();

    let mime = mime_for_key(&clean);
    let range_opt = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| parse_byte_range(s, total_len));

    let (status, start, end) = match range_opt {
        Some((s, e)) => (StatusCode::PARTIAL_CONTENT, s, e),
        None => (StatusCode::OK, 0, total_len.saturating_sub(1)),
    };

    if start > 0 {
        file.seek(SeekFrom::Start(start))
            .await
            .map_err(|e| AppError::Internal(format!("Seek error: {e}")))?;
    }

    let remaining_total = if total_len == 0 { 0 } else { (end - start) + 1 };
    let byte_stream = stream::unfold((file, remaining_total), |(mut f, rem)| async move {
        if rem == 0 {
            return None;
        }
        let to_read = (rem as usize).min(CHUNK_SIZE);
        let mut buf = vec![0_u8; to_read];
        match f.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                Some((Ok::<Bytes, std::io::Error>(Bytes::from(buf)), (f, rem - n as u64)))
            }
            Err(err) => Some((Err(err), (f, 0))),
        }
    });

    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, remaining_total.to_string())
        .header(header::CACHE_CONTROL, "public, max-age=86400");

    if status == StatusCode::PARTIAL_CONTENT {
        if let Ok(cr) = HeaderValue::from_str(&format!("bytes {start}-{end}/{total_len}")) {
            builder = builder.header(header::CONTENT_RANGE, cr);
        }
    }

    builder
        .body(Body::from_stream(byte_stream))
        .map_err(|e| AppError::Internal(format!("Response build error: {e}")))
}
