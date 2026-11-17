use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm as ArgonAlgorithm, Argon2, Params, Version,
};
use dashmap::DashMap;
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};

static VERIFIED_CRED_CACHE: OnceLock<DashMap<String, String>> = OnceLock::new();

fn cred_cache() -> &'static DashMap<String, String> {
    VERIFIED_CRED_CACHE.get_or_init(DashMap::new)
}

pub fn hash_password(password: &SecretString) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(64, 1, 1, None)
        .map_err(|e| AppError::Internal(format!("Argon2id params error: {}", e)))?;
    let argon2 = Argon2::new(ArgonAlgorithm::Argon2id, Version::V0x13, params);

    argon2
        .hash_password(password.expose_secret().as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("Argon2id hashing error: {}", e)))
}

pub fn needs_fast_rehash(stored_hash: &str) -> bool {
    !stored_hash.starts_with("$argon2id$v=19$m=64,t=1,p=1")
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
