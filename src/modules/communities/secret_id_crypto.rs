use data_encoding::BASE64URL_NOPAD;
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

use crate::modules::communities::error::CommunitiesError;

type HmacSha256 = Hmac<Sha256>;

pub const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const SECRET_DOMAIN_SALT: &[u8] = b"coffeelabs:community:secret-id:v1:";

#[derive(Debug, Clone)]
pub struct GeneratedSecretId {
    pub display_code: String,
    #[allow(dead_code)]
    pub normalized_code: String,
    pub secret_sha256: Vec<u8>,
    pub invite_url_token: String,
}

/// Menurunkan 32-byte HMAC key deterministik dari secret seed server.
pub fn derive_hmac_key(seed: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SECRET_DOMAIN_SALT);
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Mengubah 40-bit entropy menjadi 8 karakter Crockford Base32.
fn encode_40bits_crockford(bytes: &[u8; 5]) -> String {
    let acc: u64 = ((bytes[0] as u64) << 32)
        | ((bytes[1] as u64) << 24)
        | ((bytes[2] as u64) << 16)
        | ((bytes[3] as u64) << 8)
        | (bytes[4] as u64);

    let mut out = String::with_capacity(8);
    for shift in (0..8).rev() {
        let idx = ((acc >> (shift * 5)) & 0x1F) as usize;
        if let Some(&ch) = CROCKFORD_ALPHABET.get(idx) {
            out.push(ch as char);
        }
    }
    out
}

/// Menghitung 2 karakter Crockford Base32 checksum (10 bit teratas HMAC-SHA256).
fn compute_checksum_pair(payload8: &str, hmac_key: &[u8; 32]) -> Result<[char; 2], CommunitiesError> {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(hmac_key).map_err(|_| CommunitiesError::CryptoFailure)?;
    mac.update(payload8.as_bytes());
    let tag = mac.finalize().into_bytes();

    let c1 = ((tag[0] >> 3) & 0x1F) as usize;
    let c2 = ((((tag[0] & 0x07) << 2) | (tag[1] >> 6)) & 0x1F) as usize;
    let ch1 = CROCKFORD_ALPHABET
        .get(c1)
        .copied()
        .ok_or(CommunitiesError::CryptoFailure)? as char;
    let ch2 = CROCKFORD_ALPHABET
        .get(c2)
        .copied()
        .ok_or(CommunitiesError::CryptoFailure)? as char;
    Ok([ch1, ch2])
}

/// Membuat Secret ID `CLB-XXXX-XXXXXX` + SHA-256 hash + token URL 32-char.
pub fn generate_secret_id(hmac_key: &[u8; 32]) -> Result<GeneratedSecretId, CommunitiesError> {
    let mut entropy = [0u8; 5];
    OsRng.fill_bytes(&mut entropy);
    let payload8 = encode_40bits_crockford(&entropy);
    let [c1, c2] = compute_checksum_pair(&payload8, hmac_key)?;

    let mut normalized_code = String::with_capacity(10);
    normalized_code.push_str(&payload8);
    normalized_code.push(c1);
    normalized_code.push(c2);

    let display_code = format!("CLB-{}-{}{}{}", &payload8[0..4], &payload8[4..8], c1, c2);
    let secret_sha256 = Sha256::digest(normalized_code.as_bytes()).to_vec();

    let mut url_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut url_bytes);
    let invite_url_token = BASE64URL_NOPAD.encode(&url_bytes);

    Ok(GeneratedSecretId {
        display_code,
        normalized_code,
        secret_sha256,
        invite_url_token,
    })
}

/// Menormalkan typo visual (`O->0`, `I/L->1`), memverifikasi HMAC checksum di RAM,
/// lalu mengembalikan `(normalized_10_chars, sha256_bytes)`.
pub fn normalize_and_verify_secret_id(
    raw_input: &str,
    hmac_key: &[u8; 32],
) -> Option<(String, Vec<u8>)> {
    let upper_stripped: String = raw_input
        .chars()
        .map(|ch| ch.to_ascii_uppercase())
        .filter(|up| *up != '-' && *up != '_' && !up.is_ascii_whitespace())
        .collect();

    // Strip prefix `CLB` sebelum mapping `L->1` agar huruf `L` pada prefix tidak rusak.
    let without_prefix = if upper_stripped.len() > 10 {
        upper_stripped.strip_prefix("CLB").unwrap_or(&upper_stripped)
    } else {
        &upper_stripped
    };

    let mut normalized = String::with_capacity(10);
    for up in without_prefix.chars() {
        let mapped = match up {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        };
        normalized.push(mapped);
    }

    if normalized.len() != 10 {
        return None;
    }
    if !normalized
        .bytes()
        .all(|b| CROCKFORD_ALPHABET.contains(&b))
    {
        return None;
    }

    let (payload8, chk2) = normalized.split_at(8);
    let [c1, c2] = compute_checksum_pair(payload8, hmac_key).ok()?;
    let expected = [c1 as u8, c2 as u8];
    let actual = chk2.as_bytes();
    if actual.len() != 2 || (actual[0] ^ expected[0]) | (actual[1] ^ expected[1]) != 0 {
        return None;
    }

    let digest = Sha256::digest(normalized.as_bytes()).to_vec();
    Some((normalized, digest))
}

/// Mengubah `CLB-XXXX-YYYYYY` menjadi preview aman untuk non-admin atau ringkasan.
pub fn mask_secret_preview(display_code: &str) -> String {
    let chars: Vec<char> = display_code.chars().collect();
    if chars.len() <= 8 {
        return "CLB-****-******".to_string();
    }
    let suffix: String = chars[chars.len() - 3..].iter().collect();
    format!("CLB-****-***{suffix}")
}
