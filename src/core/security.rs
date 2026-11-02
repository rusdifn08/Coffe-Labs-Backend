use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm as ArgonAlgorithm, Argon2, Params, Version,
};
use dashmap::DashMap;
use ed25519_dalek::{pkcs8::EncodePrivateKey, SigningKey};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};

const LEGACY_HS256_SECRET: &[u8] = b"coffee_labs_super_secret_jwt_key_2026";
static VERIFIED_CRED_CACHE: OnceLock<DashMap<String, String>> = OnceLock::new();

fn cred_cache() -> &'static DashMap<String, String> {
    VERIFIED_CRED_CACHE.get_or_init(DashMap::new)
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub tier: String,
    pub status: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
}

#[derive(Clone)]
pub struct JwtKeyRing {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtKeyRing {
    pub fn from_seed(seed: &SecretString) -> AppResult<Self> {
        let mut hasher = Sha256::new();
        hasher.update(seed.expose_secret().as_bytes());
        let digest = hasher.finalize();

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&digest[..32]);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        let pkcs8_doc = signing_key
            .to_pkcs8_der()
            .map_err(|e| AppError::Internal(format!("Ed25519 PKCS8 error: {}", e)))?;

        let encoding = EncodingKey::from_ed_der(pkcs8_doc.as_bytes());
        let decoding = DecodingKey::from_ed_der(verifying_key.as_bytes());

        Ok(Self { encoding, decoding })
    }

    pub fn sign_access_token(&self, user_id: &str, username: &str, role: &str, tier: &str, status: &str) -> AppResult<String> {
        let now = chrono::Utc::now();
        let exp = now.checked_add_signed(chrono::Duration::days(30)).ok_or_else(|| AppError::Internal("Timestamp overflow".to_string()))?.timestamp() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
            tier: tier.to_string(),
            status: status.to_string(),
            exp,
            iat: now.timestamp() as usize,
            jti: Uuid::new_v4().to_string(),
        };
        encode(&Header::new(Algorithm::EdDSA), &claims, &self.encoding)
            .map_err(|e| AppError::Internal(format!("Ed25519 JWT sign error: {}", e)))
    }

    pub fn verify_token(&self, token: &str) -> AppResult<Claims> {
        let mut ed_val = Validation::new(Algorithm::EdDSA);
        ed_val.validate_exp = true;

        if let Ok(token_data) = decode::<Claims>(token, &self.decoding, &ed_val) {
            return Ok(token_data.claims);
        }

        // Backward compatibility for existing mobile HS256 sessions
        #[derive(Deserialize)]
        struct LegacyClaims {
            sub: String,
            username: String,
            role: String,
            status: String,
            exp: usize,
            #[serde(default)]
            tier: Option<String>,
        }

        let mut hs_val = Validation::new(Algorithm::HS256);
        hs_val.validate_exp = true;
        let legacy = decode::<LegacyClaims>(
            token,
            &DecodingKey::from_secret(LEGACY_HS256_SECRET),
            &hs_val,
        )
        .map_err(|_| {
            AppError::Unauthorized(
                "Token tidak valid atau telah kedaluwarsa. Silakan login kembali.".to_string(),
            )
        })?
        .claims;

        Ok(Claims {
            sub: legacy.sub,
            username: legacy.username,
            role: legacy.role,
            tier: legacy.tier.unwrap_or_else(|| "trial".to_string()),
            status: legacy.status,
            exp: legacy.exp,
            iat: 0,
            jti: String::new(),
        })
    }
}

pub fn hash_password(password: &SecretString) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(1024, 1, 1, None)
        .map_err(|e| AppError::Internal(format!("Argon2id params error: {}", e)))?;
    let argon2 = Argon2::new(ArgonAlgorithm::Argon2id, Version::V0x13, params);

    argon2
        .hash_password(password.expose_secret().as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("Argon2id hashing error: {}", e)))
}

pub fn needs_fast_rehash(stored_hash: &str) -> bool {
    !stored_hash.starts_with("$argon2id$v=19$m=1024,t=1,p=1")
}

pub fn cache_verified_password(password: &SecretString, stored_hash: &str) {
    let digest = compute_cred_digest(password.expose_secret(), stored_hash);
    cred_cache().insert(stored_hash.to_string(), digest);
}

fn compute_cred_digest(raw_password: &str, stored_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(stored_hash.as_bytes());
    hasher.update(b"::");
    hasher.update(raw_password.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_password(password: &SecretString, stored_hash: &str) -> bool {
    let candidate_digest = compute_cred_digest(password.expose_secret(), stored_hash);
    if let Some(cached) = cred_cache().get(stored_hash) {
        if cached.value() == &candidate_digest {
            return true;
        }
    }

    let valid = if stored_hash.starts_with("$argon2") {
        if let Ok(parsed) = PasswordHash::new(stored_hash) {
            Argon2::default()
                .verify_password(password.expose_secret().as_bytes(), &parsed)
                .is_ok()
        } else {
            false
        }
    } else {
        bcrypt::verify(password.expose_secret(), stored_hash).unwrap_or(false)
    };

    if valid {
        cred_cache().insert(stored_hash.to_string(), candidate_digest);
    }
    valid
}

pub fn generate_refresh_token() -> String {
    format!("cl_rt_{}_{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn hash_refresh_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    hex::encode(hasher.finalize())
}
