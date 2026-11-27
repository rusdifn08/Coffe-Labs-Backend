use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::modules::live_brew::error::LiveBrewError;

const HALF_MASK_19: u64 = (1_u64 << 19) - 1;
const DOMAIN_LIMIT: u64 = 100_000_000_000; // 10^11 (11 decimal payload digits)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressCodeKind {
    DynamicSession { seq_hint: u64 },
    StaticTable { seq_hint: u64 },
}

fn round_prf_19(secret: &[u8], right_half: u64, round_idx: u8) -> u64 {
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret) else {
        return right_half ^ (round_idx as u64).wrapping_mul(0x9E37);
    };
    mac.update(&right_half.to_be_bytes());
    mac.update(&[round_idx]);
    let digest = mac.finalize().into_bytes();
    let raw = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) as u64;
    raw & HALF_MASK_19
}

fn feistel_4_forward(val: u64, secret: &[u8], prefix_salt: u8) -> u64 {
    let mut left = (val >> 19) & HALF_MASK_19;
    let mut right = val & HALF_MASK_19;
    for r in 0..4_u8 {
        let f = round_prf_19(secret, right, r.wrapping_add(prefix_salt));
        let next_right = left ^ f;
        left = right;
        right = next_right;
    }
    (left << 19) | right
}

fn feistel_4_inverse(val: u64, secret: &[u8], prefix_salt: u8) -> u64 {
    let mut left = (val >> 19) & HALF_MASK_19;
    let mut right = val & HALF_MASK_19;
    for r in (0..4_u8).rev() {
        let prev_right = left;
        let f = round_prf_19(secret, prev_right, r.wrapping_add(prefix_salt));
        let prev_left = right ^ f;
        left = prev_left;
        right = prev_right;
    }
    (left << 19) | right
}

fn permute_11_digits(seq_num: u64, secret: &[u8], prefix_salt: u8) -> u64 {
    let mut current = seq_num % DOMAIN_LIMIT;
    loop {
        current = feistel_4_forward(current, secret, prefix_salt);
        if current < DOMAIN_LIMIT {
            return current;
        }
    }
}

fn unpermute_11_digits(enc: u64, secret: &[u8], prefix_salt: u8) -> u64 {
    let mut current = enc;
    loop {
        current = feistel_4_inverse(current, secret, prefix_salt);
        if current < DOMAIN_LIMIT {
            return current;
        }
    }
}

/// Menghitung 1 digit Luhn Checksum dari 12 digit prefix+payload.
pub fn compute_luhn_check_digit(twelve_digits: &str) -> u8 {
    let mut sum = 0_u32;
    let bytes = twelve_digits.as_bytes();
    for (idx_from_right, &b) in bytes.iter().rev().enumerate() {
        let d = (b.saturating_sub(b'0')) as u32;
        if idx_from_right % 2 == 0 {
            let doubled = d * 2;
            sum += if doubled > 9 { doubled - 9 } else { doubled };
        } else {
            sum += d;
        }
    }
    ((10 - (sum % 10)) % 10) as u8
}

pub fn verify_luhn_13(code: &str) -> bool {
    let bytes = code.as_bytes();
    if bytes.len() != 13 || !bytes.iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let expected = compute_luhn_check_digit(&code[..12]);
    (bytes[12] - b'0') == expected
}

fn encode_with_prefix(prefix: char, prefix_salt: u8, seq_id: u64, secret: &str) -> String {
    let payload_11 = permute_11_digits(seq_id, secret.as_bytes(), prefix_salt);
    let body_12 = format!("{prefix}{payload_11:011}");
    let check_digit = compute_luhn_check_digit(&body_12);
    format!("{body_12}{check_digit}")
}

pub fn encode_dynamic_session_code(seq_id: u64, secret: &str) -> String {
    encode_with_prefix('3', 0x10, seq_id, secret)
}

pub fn encode_static_table_code(table_seq: u64, secret: &str) -> String {
    encode_with_prefix('7', 0x40, table_seq, secret)
}

pub fn verify_and_classify_code(code: &str, secret: &str) -> Result<AddressCodeKind, LiveBrewError> {
    let trimmed = code.trim();
    if !verify_luhn_13(trimmed) {
        return Err(LiveBrewError::InvalidBrewAddressCode);
    }
    let payload: u64 = trimmed[1..12]
        .parse()
        .map_err(|_| LiveBrewError::InvalidBrewAddressCode)?;

    match trimmed.as_bytes()[0] {
        b'3' => Ok(AddressCodeKind::DynamicSession {
            seq_hint: unpermute_11_digits(payload, secret.as_bytes(), 0x10),
        }),
        b'7' => Ok(AddressCodeKind::StaticTable {
            seq_hint: unpermute_11_digits(payload, secret.as_bytes(), 0x40),
        }),
        _ => Err(LiveBrewError::InvalidBrewAddressCode),
    }
}

pub fn landing_livebrew_url(code: &str) -> String {
    let base = std::env::var("LANDING_PAGE_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://brewingcoffee.vercel.app".to_string());
    format!("{base}/livebrew/{code}")
}
