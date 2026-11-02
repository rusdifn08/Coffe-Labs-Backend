use chrono::{Duration, Utc};
use dashmap::DashMap;
use sqlx::PgPool;
use std::{
    sync::OnceLock,
    time::{Duration as StdDuration, Instant},
};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::auth::models::{RefreshTokenSession, UserRow},
};

const USER_COLS: &str = "id, username, password_hash, full_name, role, status, tier, avatar_url, created_at, updated_at";
static USER_ROW_CACHE: OnceLock<DashMap<String, (UserRow, Instant)>> = OnceLock::new();

fn user_cache() -> &'static DashMap<String, (UserRow, Instant)> {
    USER_ROW_CACHE.get_or_init(DashMap::new)
}

pub fn put_user_cache(user: &UserRow) {
    user_cache().insert(user.username.to_lowercase(), (user.clone(), Instant::now()));
}

pub fn invalidate_user_cache_all() {
    user_cache().clear();
}

pub async fn preload_users_cache(pool: &PgPool) {
    let q = format!("SELECT {} FROM users LIMIT 500", USER_COLS);
    if let Ok(rows) = sqlx::query_as::<_, UserRow>(&q).fetch_all(pool).await {
        let now = Instant::now();
        for u in rows {
            user_cache().insert(u.username.to_lowercase(), (u, now));
        }
    }
}

pub async fn exists_by_username(pool: &PgPool, username: &str) -> AppResult<bool> {
    let key = username.to_lowercase();
    if let Some(entry) = user_cache().get(&key) {
        if entry.value().1.elapsed() < StdDuration::from_secs(180) {
            return Ok(true);
        }
    }
    let exists = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(&key)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn find_by_username(pool: &PgPool, username: &str) -> AppResult<Option<UserRow>> {
    let key = username.to_lowercase();
    if let Some(entry) = user_cache().get(&key) {
        if entry.value().1.elapsed() < StdDuration::from_secs(180) {
            return Ok(Some(entry.value().0.clone()));
        }
    }
    let q = format!("SELECT {} FROM users WHERE username = $1", USER_COLS);
    let user = sqlx::query_as::<_, UserRow>(&q)
        .bind(&key)
        .fetch_optional(pool)
        .await?;
    if let Some(ref u) = user {
        put_user_cache(u);
    }
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
    put_user_cache(&user);
    Ok(user)
}

pub async fn update_password(pool: &PgPool, user_id: Uuid, new_hash: &str) -> AppResult<()> {
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_hash)
        .bind(user_id)
        .execute(pool)
        .await?;
    for mut item in user_cache().iter_mut() {
        if item.value().0.id == user_id {
            item.value_mut().0.password_hash = new_hash.to_string();
            item.value_mut().1 = Instant::now();
        }
    }
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
