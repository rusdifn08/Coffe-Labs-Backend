use chrono::{Duration, Utc};
use rand::{rngs::OsRng, Rng};
use secrecy::SecretString;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use tracing::warn;
use uuid::Uuid;

use crate::{
    core::{
        email_service::{mask_email, send_password_changed_notice, send_password_reset_code_email},
        error::{AppError, AppResult},
        security::{cache_verified_password, hash_password},
    },
    modules::auth::{
        dto::{ForgotPasswordRequest, ForgotPasswordResponse, ResetPasswordRequest},
        password_reset_repo as reset_repo, repository,
    },
};

const RESET_TTL_MINUTES: i64 = 15;
const RESEND_COOLDOWN_SECONDS: i64 = 60;
const MAX_VERIFY_ATTEMPTS: i32 = 5;

pub fn generate_6_digit_code() -> String {
    let n: u32 = OsRng.gen_range(0..1_000_000);
    format!("{:06}", n)
}

pub fn hash_reset_code(user_id: Uuid, code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    hasher.update(b"::coffee_labs_reset::");
    hasher.update(code.trim().as_bytes());
    hex::encode(hasher.finalize())
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len() && bool::from(a.as_bytes().ct_eq(b.as_bytes()))
}

pub async fn request_password_reset(
    pool: &PgPool,
    req: ForgotPasswordRequest,
) -> AppResult<ForgotPasswordResponse> {
    let identifier = req.identifier.trim();
    let user = reset_repo::find_user_by_identifier(pool, identifier)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(
                "Akun dengan alamat email atau username tersebut tidak ditemukan.".to_string(),
            )
        })?;

    let email = user
        .email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .ok_or_else(|| {
            AppError::BadRequest(
                "Akun ini belum memiliki email terdaftar. Silakan tambahkan email pada menu Pengaturan Profil atau hubungi Admin Coffee Labs.".to_string(),
            )
        })?
        .to_lowercase();

    if let Some(secs) = reset_repo::seconds_since_last_reset(pool, user.id).await? {
        if secs >= 0 && secs < RESEND_COOLDOWN_SECONDS {
            let wait = RESEND_COOLDOWN_SECONDS - secs;
            return Err(AppError::RateLimited(format!(
                "Kode reset baru saja dikirim. Tunggu {} detik sebelum meminta kode lagi.",
                wait
            )));
        }
    }

    let code = generate_6_digit_code();
    let code_hash = hash_reset_code(user.id, &code);
    let expires_at = Utc::now() + Duration::minutes(RESET_TTL_MINUTES);

    reset_repo::invalidate_pending_tokens(pool, user.id).await?;
    let token_id = reset_repo::insert_reset_token(pool, user.id, &email, &code_hash, expires_at).await?;

    if let Err(err) = send_password_reset_code_email(
        &email,
        &user.full_name,
        &user.username,
        &code,
        RESET_TTL_MINUTES,
    )
    .await
    {
        let _ = reset_repo::delete_reset_token(pool, token_id).await;
        return Err(err);
    }

    Ok(ForgotPasswordResponse {
        masked_email: mask_email(&email),
        expires_in_seconds: RESET_TTL_MINUTES * 60,
    })
}

pub async fn confirm_password_reset(pool: &PgPool, req: ResetPasswordRequest) -> AppResult<()> {
    let clean_code = req.code.trim();
    if clean_code.len() != 6 || !clean_code.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Validation("Kode reset harus berupa 6 digit angka.".to_string()));
    }

    let user = reset_repo::find_user_by_identifier(pool, req.identifier.trim())
        .await?
        .ok_or_else(|| AppError::NotFound("Akun tidak ditemukan.".to_string()))?;

    let token = reset_repo::find_latest_active_token(pool, user.id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest(
                "Kode reset tidak ditemukan atau sudah digunakan. Silakan minta kode baru.".to_string(),
            )
        })?;

    if token.expires_at < Utc::now() {
        let _ = reset_repo::mark_token_used(pool, token.id).await;
        return Err(AppError::BadRequest(
            "Kode reset telah kedaluwarsa (lebih dari 15 menit). Silakan minta kode baru.".to_string(),
        ));
    }

    if token.attempts >= MAX_VERIFY_ATTEMPTS {
        let _ = reset_repo::mark_token_used(pool, token.id).await;
        return Err(AppError::RateLimited(
            "Terlalu banyak percobaan kode salah. Silakan minta kode reset baru.".to_string(),
        ));
    }

    let candidate_hash = hash_reset_code(user.id, clean_code);
    if !constant_time_eq(&candidate_hash, &token.code_hash) {
        let _ = reset_repo::increment_attempts(pool, token.id).await;
        let remaining = (MAX_VERIFY_ATTEMPTS - token.attempts - 1).max(0);
        return Err(AppError::BadRequest(format!(
            "Kode reset tidak sesuai. Sisa percobaan: {}x.",
            remaining
        )));
    }

    let new_secret = SecretString::from(req.new_password);
    let new_hash = hash_password(&new_secret)?;
    cache_verified_password(&new_secret, &new_hash);

    repository::update_password(pool, user.id, &new_hash).await?;
    reset_repo::mark_token_used(pool, token.id).await?;
    reset_repo::revoke_all_user_refresh_tokens(pool, user.id).await?;
    repository::invalidate_user_cache_all();

    let notify_email = token.email.clone();
    let full_name = user.full_name.clone();
    let username = user.username.clone();
    tokio::spawn(async move {
        if let Err(err) = send_password_changed_notice(&notify_email, &full_name, &username).await {
            warn!("Notifikasi konfirmasi ubah password tidak terkirim: {}", err);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_six_digit_numeric_code() {
        for _ in 0..32 {
            let code = generate_6_digit_code();
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn hashes_and_compares_in_constant_time() {
        let uid = Uuid::new_v4();
        let h1 = hash_reset_code(uid, "123456");
        let h2 = hash_reset_code(uid, "123456");
        let h3 = hash_reset_code(uid, "654321");
        assert!(constant_time_eq(&h1, &h2));
        assert!(!constant_time_eq(&h1, &h3));
    }
}
