use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::auth::models::{RefreshTokenSession, UserRow},
};

const USER_COLS: &str = "id, username, password_hash, full_name, role, status, tier, avatar_url, created_at, updated_at";

pub async fn exists_by_username(pool: &PgPool, username: &str) -> AppResult<bool> {
    let exists = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(username)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn find_by_username(pool: &PgPool, username: &str) -> AppResult<Option<UserRow>> {
    let q = format!("SELECT {} FROM users WHERE username = $1", USER_COLS);
    let user = sqlx::query_as::<_, UserRow>(&q)
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn find_by_id(pool: &PgPool, user_id: Uuid) -> AppResult<Option<UserRow>> {
    let q = format!("SELECT {} FROM users WHERE id = $1", USER_COLS);
    let user = sqlx::query_as::<_, UserRow>(&q)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
    full_name: &str,
    avatar_url: &str,
) -> AppResult<UserRow> {
    let q = format!(
        "INSERT INTO users (username, password_hash, full_name, role, status, tier, avatar_url)
         VALUES ($1, $2, $3, 'user', 'approved', 'trial', $4)
         RETURNING {}",
        USER_COLS
    );
    let user = sqlx::query_as::<_, UserRow>(&q)
        .bind(username)
        .bind(password_hash)
        .bind(full_name)
        .bind(avatar_url)
        .fetch_one(pool)
        .await?;
    Ok(user)
}

pub async fn update_password(pool: &PgPool, user_id: Uuid, new_hash: &str) -> AppResult<()> {
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    family_id: Uuid,
    token_hash: &str,
    device_id: &str,
    device_name: &str,
    ip_address: Option<&str>,
) -> AppResult<()> {
    let expires_at = Utc::now() + Duration::days(60);
    sqlx::query(
        r#"INSERT INTO auth_refresh_tokens (
            user_id, family_id, token_hash, device_id, device_name, ip_address, expires_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(user_id)
    .bind(family_id)
    .bind(token_hash)
    .bind(device_id)
    .bind(device_name)
    .bind(ip_address)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> AppResult<Option<RefreshTokenSession>> {
    let row = sqlx::query_as::<_, RefreshTokenSession>(
        "SELECT * FROM auth_refresh_tokens WHERE token_hash = $1",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn revoke_token_by_id(pool: &PgPool, id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE auth_refresh_tokens SET is_revoked = TRUE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_entire_family(pool: &PgPool, family_id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE auth_refresh_tokens SET is_revoked = TRUE WHERE family_id = $1")
        .bind(family_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_active_sessions(
    pool: &PgPool,
    user_id: Uuid,
) -> AppResult<Vec<RefreshTokenSession>> {
    let rows = sqlx::query_as::<_, RefreshTokenSession>(
        "SELECT * FROM auth_refresh_tokens WHERE user_id = $1 AND is_revoked = FALSE AND expires_at > NOW() ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
