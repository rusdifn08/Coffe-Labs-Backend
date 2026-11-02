use chrono::Utc;
use secrecy::SecretString;
use sqlx::PgPool;
use tracing::warn;
use uuid::Uuid;

use crate::{
    core::{
        error::{AppError, AppResult},
        security::{
            cache_verified_password, generate_refresh_token, hash_password, hash_refresh_token,
            needs_fast_rehash, verify_password, JwtKeyRing,
        },
    },
    modules::auth::{
        dto::{
            AuthSessionPayload, ChangePasswordRequest, LoginRequest, RefreshTokenRequest,
            RegisterRequest, UnifiedAuthResponse,
        },
        error::AuthDomainError,
        models::{RefreshTokenSession, UserRow, UserSummary},
        repository,
    },
};

fn build_auth_response(
    user: UserRow,
    token: String,
    refresh_token: String,
    msg: &str,
) -> UnifiedAuthResponse {
    let summary: UserSummary = user.into();
    let payload = AuthSessionPayload {
        token: token.clone(),
        refresh_token: refresh_token.clone(),
        user: summary.clone(),
        message: msg.to_string(),
    };
    UnifiedAuthResponse {
        success: true,
        message: msg.to_string(),
        token,
        refresh_token,
        user: summary,
        data: payload,
    }
}

pub async fn register_user(
    pool: &PgPool,
    keyring: &JwtKeyRing,
    req: RegisterRequest,
) -> AppResult<(UserSummary, String, String)> {
    let username = req.username.trim().to_lowercase();
    let full_name = req.full_name.trim();

    if repository::exists_by_username(pool, &username).await? {
        return Err(AuthDomainError::UsernameTaken(username).into());
    }

    let secret_pwd = SecretString::from(req.password);
    let pwd_hash = hash_password(&secret_pwd)?;
    cache_verified_password(&secret_pwd, &pwd_hash);
    let avatar = req.avatar_url.unwrap_or_else(|| "assets/images/characters/avatar_1.png".to_string());

    let user = repository::create_user(pool, &username, &pwd_hash, full_name, &avatar).await?;
    let token = keyring.sign_access_token(&user.id.to_string(), &user.username, &user.role, &user.tier, &user.status)?;

    let raw_refresh = generate_refresh_token();
    let rt_hash = hash_refresh_token(&raw_refresh);
    let device_id = req.device_id.unwrap_or_else(|| "mobile_default".to_string());
    let device_name = req.device_name.unwrap_or_else(|| "Coffee Labs Mobile".to_string());
    let pool_bg = pool.clone();
    let uid = user.id;

    tokio::spawn(async move {
        let _ = repository::insert_refresh_token(&pool_bg, uid, Uuid::new_v4(), &rt_hash, &device_id, &device_name, None).await;
    });
    Ok((user.into(), token, raw_refresh))
}

pub async fn login_user(
    pool: &PgPool,
    keyring: &JwtKeyRing,
    req: LoginRequest,
) -> AppResult<UnifiedAuthResponse> {
    let username = req.username.trim().to_lowercase();
    let user = repository::find_by_username(pool, &username)
        .await?
        .ok_or(AuthDomainError::InvalidCredentials)?;

    let secret_pwd = SecretString::from(req.password);
    if !verify_password(&secret_pwd, &user.password_hash) {
        return Err(AuthDomainError::InvalidCredentials.into());
    }

    if user.role != "admin" && user.status == "declined" {
        return Err(AppError::AccountStatus {
            status: "declined".to_string(),
            message: "Akun Anda telah dinonaktifkan oleh admin. Silakan hubungi admin.".to_string(),
        });
    }

    let token = keyring.sign_access_token(&user.id.to_string(), &user.username, &user.role, &user.tier, &user.status)?;
    let raw_refresh = generate_refresh_token();
    let rt_hash = hash_refresh_token(&raw_refresh);
    let device_id = req.device_id.unwrap_or_else(|| "mobile_default".to_string());
    let device_name = req.device_name.unwrap_or_else(|| "Coffee Labs Mobile".to_string());

    let should_rehash = needs_fast_rehash(&user.password_hash);
    let pool_bg = pool.clone();
    let uid = user.id;

    tokio::spawn(async move {
        if should_rehash {
            if let Ok(fast_hash) = hash_password(&secret_pwd) {
                cache_verified_password(&secret_pwd, &fast_hash);
                let _ = repository::update_password(&pool_bg, uid, &fast_hash).await;
            }
        }
        let _ = repository::insert_refresh_token(&pool_bg, uid, Uuid::new_v4(), &rt_hash, &device_id, &device_name, None).await;
    });

    Ok(build_auth_response(user, token, raw_refresh, "Login berhasil!"))
}

pub async fn rotate_refresh_token(
    pool: &PgPool,
    keyring: &JwtKeyRing,
    req: RefreshTokenRequest,
) -> AppResult<UnifiedAuthResponse> {
    let rt_hash = hash_refresh_token(req.refresh_token.trim());
    let session = repository::find_refresh_token(pool, &rt_hash)
        .await?
        .ok_or(AuthDomainError::InvalidRefreshToken)?;

    if session.is_revoked {
        warn!("🚨 Security Alert: Refresh token reuse for user_id={}, family_id={}", session.user_id, session.family_id);
        repository::revoke_entire_family(pool, session.family_id).await?;
        return Err(AuthDomainError::TokenReuseDetected.into());
    }

    if session.expires_at < Utc::now() {
        repository::revoke_token_by_id(pool, session.id).await?;
        return Err(AuthDomainError::InvalidRefreshToken.into());
    }

    repository::revoke_token_by_id(pool, session.id).await?;
    let user = repository::find_by_id(pool, session.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User tidak ditemukan.".to_string()))?;

    let new_access = keyring.sign_access_token(&user.id.to_string(), &user.username, &user.role, &user.tier, &user.status)?;
    let new_refresh = generate_refresh_token();
    let new_hash = hash_refresh_token(&new_refresh);
    let device_id = req.device_id.as_deref().unwrap_or(&session.device_id);

    repository::insert_refresh_token(
        pool, user.id, session.family_id, &new_hash, device_id, &session.device_name, session.ip_address.as_deref(),
    ).await?;

    Ok(build_auth_response(user, new_access, new_refresh, "Token berhasil diperbarui."))
}

pub async fn change_user_password(
    pool: &PgPool,
    user_id: Uuid,
    req: ChangePasswordRequest,
) -> AppResult<()> {
    let user = repository::find_by_id(pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User tidak ditemukan.".to_string()))?;

    let curr_secret = SecretString::from(req.current_password);
    if !verify_password(&curr_secret, &user.password_hash) {
        return Err(AuthDomainError::CurrentPasswordMismatch.into());
    }

    let new_secret = SecretString::from(req.new_password);
    let new_hash = hash_password(&new_secret)?;
    repository::update_password(pool, user_id, &new_hash).await
}

pub async fn get_active_sessions(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<RefreshTokenSession>> {
    repository::list_active_sessions(pool, user_id).await
}
