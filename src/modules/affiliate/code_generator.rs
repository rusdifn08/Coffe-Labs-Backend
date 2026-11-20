use rand::Rng;
use rustrict::CensorStr;

use crate::modules::affiliate::error::AffiliateError;

/// Alfabet ramah baca Crockford (tanpa 0/O dan 1/I/L agar tidak salah ketik di TikTok/IG).
const CROCKFORD_CHARS: &[u8] = b"23456789ABCDEFGHJKMNPQRSTVWXYZ";

const RESERVED_CODES: &[&str] = &[
    "ADMIN",
    "COFFEELABS",
    "OFFICIAL",
    "SUPPORT",
    "SYSTEM",
    "STAFF",
    "OWNER",
    "FREE",
    "PROMO",
    "TEST",
];

/// Menghasilkan kode referral unik otomatis dari username Pro (misal `CL-RISA7K` atau `CL-8F3K9M`).
pub fn generate_default_affiliate_code(username: &str) -> String {
    let prefix: String = username
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| !matches!(c, 'O' | 'I' | 'L' | '0' | '1'))
        .take(4)
        .collect();

    let mut rng = rand::thread_rng();
    let suffix_len = if prefix.len() >= 3 { 4 } else { 6 };
    let mut suffix = String::with_capacity(suffix_len);
    for _ in 0..suffix_len {
        let idx = rng.gen_range(0..CROCKFORD_CHARS.len());
        suffix.push(CROCKFORD_CHARS[idx] as char);
    }

    if prefix.len() >= 3 {
        format!("CL-{prefix}{suffix}")
    } else {
        format!("CL-{suffix}")
    }
}

/// Normalisasi & validasi kode vanity kustom (4..=16 karakter alfanumerik, anti kata kasar & reserved).
pub fn validate_and_normalize_custom_code(raw: &str) -> Result<String, AffiliateError> {
    let cleaned = raw.trim().to_ascii_uppercase();
    if cleaned.len() < 4 || cleaned.len() > 16 {
        return Err(AffiliateError::InvalidCodeFormat(
            "Kode referral kustom harus terdiri dari 4 sampai 16 karakter.".into(),
        ));
    }
    if !cleaned
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(AffiliateError::InvalidCodeFormat(
            "Kode referral hanya boleh menggunakan huruf kapital A-Z, angka 0-9, atau tanda hubung (-).".into(),
        ));
    }
    if cleaned.starts_with('-') || cleaned.ends_with('-') || cleaned.contains("--") {
        return Err(AffiliateError::InvalidCodeFormat(
            "Kode referral tidak boleh diawali/diakhiri tanda hubung (-).".into(),
        ));
    }
    if RESERVED_CODES.contains(&cleaned.as_str()) {
        return Err(AffiliateError::InvalidCodeFormat(
            "Kode tersebut merupakan nama tercadang sistem Coffee Labs.".into(),
        ));
    }
    if cleaned.is_inappropriate() {
        return Err(AffiliateError::InvalidCodeFormat(
            "Kode referral mengandung kata yang tidak pantas.".into(),
        ));
    }
    Ok(cleaned)
}

/// Membuat nomor tiket penarikan unik `WD-YYYYMMDD-XXXX`.
pub fn generate_withdrawal_ticket(now: chrono::DateTime<chrono::Utc>) -> String {
    let mut rng = rand::thread_rng();
    let mut code = String::with_capacity(4);
    for _ in 0..4 {
        let idx = rng.gen_range(0..CROCKFORD_CHARS.len());
        code.push(CROCKFORD_CHARS[idx] as char);
    }
    format!("WD-{}-{code}", now.format("%Y%m%d"))
}

pub fn is_pro_eligible_tier(tier: &str, role: &str) -> bool {
    role.eq_ignore_ascii_case("admin")
        || tier.eq_ignore_ascii_case("pro")
        || tier.eq_ignore_ascii_case("business")
}
