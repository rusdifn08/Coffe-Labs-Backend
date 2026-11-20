use std::sync::OnceLock;

use chrono::Utc;
use dashmap::DashMap;
use fred::clients::RedisPool;
use fred::interfaces::KeysInterface;
use fred::types::{Expiration, SetOptions};
use hmac::{Hmac, Mac};
use regex::Regex;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::core::error::AppResult;
use crate::modules::qris_billing::error::QrisBillingError;

type HmacSha256 = Hmac<Sha256>;

const MAX_TIMESTAMP_DRIFT_MS: i64 = 60_000;
const NONCE_TTL_SECS: i64 = 120;

static LOCAL_NONCE_CACHE: OnceLock<DashMap<String, i64>> = OnceLock::new();
static RUPIAH_REGEX: OnceLock<Option<Regex>> = OnceLock::new();

fn nonce_cache() -> &'static DashMap<String, i64> {
    LOCAL_NONCE_CACHE.get_or_init(DashMap::new)
}

/// Menghitung tanda tangan hex lowercase `HMAC-SHA256(secret, "{timestamp}.{nonce}.{raw_body}")`.
pub fn compute_bridge_signature_hex(
    secret: &str,
    timestamp_ms: i64,
    nonce: &str,
    raw_body: &[u8],
) -> AppResult<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| QrisBillingError::BridgeSignatureInvalid)?;
    mac.update(timestamp_ms.to_string().as_bytes());
    mac.update(b".");
    mac.update(nonce.as_bytes());
    mac.update(b".");
    mac.update(raw_body);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Verifikasi Zero-Trust: Timestamp <= 60s, Nonce anti-replay (`SET NX EX 120`), & Constant-Time HMAC-SHA256.
pub async fn verify_bridge_request(
    redis_pool: Option<&RedisPool>,
    secret: &str,
    timestamp_header: Option<&str>,
    nonce_header: Option<&str>,
    signature_header: Option<&str>,
    raw_body: &[u8],
) -> AppResult<()> {
    let ts_raw = timestamp_header.ok_or(QrisBillingError::BridgeTimestampExpired)?;
    let ts_ms: i64 = ts_raw
        .trim()
        .parse()
        .map_err(|_| QrisBillingError::BridgeTimestampExpired)?;
    let now_ms = Utc::now().timestamp_millis();
    if (now_ms - ts_ms).abs() > MAX_TIMESTAMP_DRIFT_MS {
        return Err(QrisBillingError::BridgeTimestampExpired.into());
    }

    let nonce = nonce_header
        .map(str::trim)
        .filter(|s| s.len() >= 8 && s.len() <= 128)
        .ok_or(QrisBillingError::BridgeReplayDetected)?;
    let provided_sig = signature_header
        .map(str::trim)
        .filter(|s| s.len() == 64)
        .ok_or(QrisBillingError::BridgeSignatureInvalid)?;

    let expected_sig = compute_bridge_signature_hex(secret, ts_ms, nonce, raw_body)?;
    let provided_bytes = hex::decode(provided_sig)
        .map_err(|_| QrisBillingError::BridgeSignatureInvalid)?;
    let expected_bytes = hex::decode(&expected_sig)
        .map_err(|_| QrisBillingError::BridgeSignatureInvalid)?;

    let is_valid = provided_bytes.len() == expected_bytes.len()
        && bool::from(provided_bytes.ct_eq(&expected_bytes));
    if !is_valid {
        return Err(QrisBillingError::BridgeSignatureInvalid.into());
    }

    guard_nonce_replay(redis_pool, nonce, now_ms).await?;
    Ok(())
}

async fn guard_nonce_replay(
    redis_pool: Option<&RedisPool>,
    nonce: &str,
    now_ms: i64,
) -> AppResult<()> {
    if let Some(pool) = redis_pool {
        let key = format!("qris:bridge_nonce:{nonce}");
        let set_res: Result<Option<String>, _> = pool
            .set(
                &key,
                "1",
                Some(Expiration::EX(NONCE_TTL_SECS)),
                Some(SetOptions::NX),
                false,
            )
            .await;
        if let Ok(val) = set_res {
            if val.is_none() {
                return Err(QrisBillingError::BridgeReplayDetected.into());
            }
            return Ok(());
        }
    }

    let cache = nonce_cache();
    let cutoff = now_ms - (NONCE_TTL_SECS * 1000);
    cache.retain(|_, exp| *exp >= cutoff);
    if cache.insert(nonce.to_string(), now_ms).is_some() {
        return Err(QrisBillingError::BridgeReplayDetected.into());
    }
    Ok(())
}

/// Mengekstrak nominal Rupiah Indonesia eksak dari teks notifikasi Livin' Merchant / email Mandiri.
/// Mendukung: `"Rp49.317"`, `"Rp 149.842,00"`, `"IDR 490.512"`, `"Rp1.490.123"`.
pub fn parse_indonesian_rupiah_amount(text: &str) -> Option<i64> {
    let re = RUPIAH_REGEX
        .get_or_init(|| Regex::new(r"(?i)(?:Rp\.?|IDR)\s*([0-9][0-9\.,]{0,18})").ok())
        .as_ref()?;
    let caps = re.captures(text)?;
    let raw_num = caps.get(1)?.as_str().trim();

    // Hapus sen desimal seperti `,00` atau `.00` di akhir string
    let whole_part = if let Some(stripped) = raw_num.strip_suffix(",00") {
        stripped
    } else if let Some(stripped) = raw_num.strip_suffix(".00") {
        stripped
    } else if let Some((before_comma, after_comma)) = raw_num.rsplit_once(',') {
        if after_comma.len() <= 2 {
            before_comma
        } else {
            raw_num
        }
    } else {
        raw_num
    };

    let digits_only: String = whole_part.chars().filter(|c| c.is_ascii_digit()).collect();
    digits_only.parse::<i64>().ok().filter(|&v| v > 0)
}

/// Membuat sidik jari idempoten SHA-256 untuk mencegah double-spend antara Channel A & Channel B.
pub fn compute_mutation_fingerprint(
    amount_idr: i64,
    rrn: Option<&str>,
    received_at_epoch_sec: i64,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(amount_idr.to_string().as_bytes());
    hasher.update(b"|");
    if let Some(clean_rrn) = rrn.map(str::trim).filter(|s| !s.is_empty()) {
        hasher.update(b"rrn:");
        hasher.update(clean_rrn.as_bytes());
    } else {
        let bucket_3min = received_at_epoch_sec / 180;
        hasher.update(b"bucket:");
        hasher.update(bucket_3min.to_string().as_bytes());
    }
    hex::encode(hasher.finalize())
}
