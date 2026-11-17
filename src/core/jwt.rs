use std::{fmt, sync::Arc};

use chrono::Utc;
use jsonwebtoken::{
    decode, encode, errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::error::AppError;

fn default_tier() -> String {
    "Trial".to_string()
}

fn default_role() -> String {
    "user".to_string()
}

fn default_username() -> String {
    String::new()
}

fn default_status() -> String {
    "active".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JwtClaims {
    pub sub: Uuid,
    #[serde(default = "default_tier")]
    pub tier: String,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default = "default_status")]
    pub status: String,
    pub iat: i64,
    pub exp: i64,
    #[serde(default)]
    pub jti: Option<Uuid>,
}

struct JwtManagerInner {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    expiration_secs: u64,
}

impl fmt::Debug for JwtManagerInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtManagerInner")
            .field("algorithm", &"HS256")
            .field("expiration_secs", &self.expiration_secs)
            .field("keys", &"[REDACTED]")
            .finish()
    }
}

/// Manajer JWT stateless berbasis HMAC-SHA256 (`HS256`) dengan key pre-computed di startup.
#[derive(Clone, Debug)]
pub struct JwtManager {
    inner: Arc<JwtManagerInner>,
}

impl JwtManager {
    /// Membangun `JwtManager` dari `JWT_SECRET` dan TTL detik tanpa fallback rahasia hardcoded.
    pub fn new(secret: &str, expiration_secs: u64) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 30;

        let effective_ttl = if expiration_secs == 0 {
            86_400
        } else {
            expiration_secs
        };

        Self {
            inner: Arc::new(JwtManagerInner {
                encoding_key: EncodingKey::from_secret(secret.as_bytes()),
                decoding_key: DecodingKey::from_secret(secret.as_bytes()),
                validation,
                expiration_secs: effective_ttl,
            }),
        }
    }

    pub fn expiration_seconds(&self) -> u64 {
        self.inner.expiration_secs
    }

    /// Menerbitkan access token HS256 untuk `user_id` dan `account_tier` dengan TTL konfigurasi.
    pub fn generate_token(&self, user_id: Uuid, account_tier: &str) -> Result<String, AppError> {
        self.generate_user_token(user_id, "", "user", account_tier, "active")
    }

    /// Menerbitkan access token HS256 lengkap dengan metadata profil RBAC (`username`, `role`, `status`).
    pub fn generate_user_token(
        &self,
        user_id: Uuid,
        username: &str,
        role: &str,
        account_tier: &str,
        status: &str,
    ) -> Result<String, AppError> {
        let ttl = i64::try_from(self.inner.expiration_secs).unwrap_or(86_400);
        self.generate_token_with_ttl(user_id, username, role, account_tier, status, ttl)
    }

    /// Menerbitkan token dengan TTL kustom (positif atau negatif untuk pengujian kedaluwarsa).
    pub fn generate_token_with_ttl(
        &self,
        user_id: Uuid,
        username: &str,
        role: &str,
        account_tier: &str,
        status: &str,
        ttl_secs: i64,
    ) -> Result<String, AppError> {
        let now = Utc::now().timestamp();
        let tier_val = if account_tier.trim().is_empty() {
            default_tier()
        } else {
            account_tier.trim().to_string()
        };
        let role_val = if role.trim().is_empty() {
            default_role()
        } else {
            role.trim().to_string()
        };
        let status_val = if status.trim().is_empty() {
            default_status()
        } else {
            status.trim().to_string()
        };

        let claims = JwtClaims {
            sub: user_id,
            tier: tier_val,
            role: role_val,
            username: username.trim().to_string(),
            status: status_val,
            iat: now,
            exp: now.saturating_add(ttl_secs),
            jti: Some(Uuid::new_v4()),
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.inner.encoding_key).map_err(|err| {
            AppError::Internal(format!("Gagal menandatangani token JWT HS256: {err}"))
        })
    }

    /// Memverifikasi tanda tangan HS256 dan masa berlaku (`exp`) token JWT.
    pub fn verify_token(&self, token: &str) -> Result<JwtClaims, AppError> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(AppError::Unauthorized(
                "Token JWT kosong atau tidak valid.".to_string(),
            ));
        }

        decode::<JwtClaims>(trimmed, &self.inner.decoding_key, &self.inner.validation)
            .map(|data| data.claims)
            .map_err(|err| match err.kind() {
                ErrorKind::ExpiredSignature => AppError::Unauthorized(
                    "Sesi Anda telah kedaluwarsa. Silakan login kembali.".to_string(),
                ),
                _ => AppError::Unauthorized(
                    "Token JWT tidak valid atau tanda tangan ditolak.".to_string(),
                ),
            })
    }
}
