use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

pub use crate::modules::barista_toolkit::bitmask_math::{
    decode_bitmap_13, popcount_100bit, BITMAP_BYTES_LEN,
};
use crate::modules::barista_toolkit::bitmask_math::merge_100bit_segments;

pub const MIN_COMPLETION_PCT: i16 = 85;
pub const MIN_QUIZ_SCORE_PCT: i16 = 80;

type HmacSha256 = Hmac<Sha256>;

pub fn merge_bitmap_or(
    existing: &[u8],
    incoming: &[u8; BITMAP_BYTES_LEN],
) -> [u8; BITMAP_BYTES_LEN] {
    let (merged, _) = merge_100bit_segments(existing, incoming);
    merged
}

pub fn grade_quiz_answers(
    quiz_questions: &serde_json::Value,
    answers: &[usize],
) -> Option<i16> {
    let arr = quiz_questions.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut correct = 0_usize;
    for (idx, q) in arr.iter().enumerate() {
        let expected = q.get("answer_idx").and_then(|v| v.as_u64()).map(|v| v as usize);
        if expected.is_some_and(|e| answers.get(idx) == Some(&e)) {
            correct += 1;
        }
    }
    let pct = ((correct as f64 / arr.len() as f64) * 100.0).round() as i16;
    Some(pct.clamp(0, 100))
}

pub fn evaluate_completion(
    completion_pct: i16,
    quiz_questions: &serde_json::Value,
    quiz_score: Option<i16>,
) -> bool {
    let has_quiz = quiz_questions.as_array().is_some_and(|a| !a.is_empty());
    let video_ok = completion_pct >= MIN_COMPLETION_PCT;
    let quiz_ok = !has_quiz || quiz_score.is_some_and(|s| s >= MIN_QUIZ_SCORE_PCT);
    video_ok && quiz_ok
}

pub fn sign_hls_manifest_url(
    manifest_path: &str,
    user_id: Uuid,
    signing_secret: &str,
) -> (String, chrono::DateTime<Utc>) {
    let expires_at = Utc::now() + Duration::minutes(30);
    let exp_epoch = expires_at.timestamp();
    let msg = format!("{manifest_path}|{user_id}|{exp_epoch}");

    let sig_hex = match HmacSha256::new_from_slice(signing_secret.as_bytes()) {
        Ok(mut mac) => {
            mac.update(msg.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        Err(_) => String::new(),
    };

    let clean_path = manifest_path.trim_start_matches('/');
    let url = format!(
        "https://cdn.coffeelabs.app/{clean_path}?uid={user_id}&exp={exp_epoch}&sig={sig_hex}"
    );
    (url, expires_at)
}
