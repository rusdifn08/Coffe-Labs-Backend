use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::auth::{models::UserRow, repository::USER_COLS},
};

#[derive(Debug, FromRow, Clone)]
pub struct PasswordResetTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub code_hash: String,
    pub attempts: i32,
    pub expires_at: DateTime<Utc>,
}

pub async fn exists_by_email(
    pool: &PgPool,
    email: &str,
    exclude_user_id: Option<Uuid>,
) -> AppResult<bool> {
    let key = email.trim().to_lowercase();
    let exists: bool = match exclude_user_id {
        Some(uid) => {
            sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = $1 AND id <> $2)",
            )
            .bind(&key)
            .bind(uid)
            .fetch_one(pool)
            .await?
        }
        None => {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = $1)")
                .bind(&key)
                .fetch_one(pool)
                .await?
        }
    };
    Ok(exists)
}

pub async fn find_user_by_identifier(pool: &PgPool, identifier: &str) -> AppResult<Option<UserRow>> {
    let key = identifier.trim().to_lowercase();
    let q = format!(
        "SELECT {} FROM users WHERE LOWER(username) = $1 OR LOWER(email) = $1 LIMIT 1",
        USER_COLS
    );
    let user = sqlx::query_as::<_, UserRow>(&q)
        .bind(&key)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn seconds_since_last_reset(pool: &PgPool, user_id: Uuid) -> AppResult<Option<i64>> {
    let secs: Option<i64> = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (NOW() - created_at))::BIGINT
         FROM password_reset_tokens
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(secs)
}

pub async fn invalidate_pending_tokens(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        "UPDATE password_reset_tokens SET used_at = NOW() WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    email: &str,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO password_reset_tokens (user_id, email, code_hash, expires_at)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(user_id)
    .bind(email)
    .bind(code_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn delete_reset_token(pool: &PgPool, token_id: Uuid) -> AppResult<()> {
    sqlx::query("DELETE FROM password_reset_tokens WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn find_latest_active_token(
    pool: &PgPool,
    user_id: Uuid,
) -> AppResult<Option<PasswordResetTokenRow>> {
    let row = sqlx::query_as::<_, PasswordResetTokenRow>(
        "SELECT id, user_id, email, code_hash, attempts, expires_at
         FROM password_reset_tokens
         WHERE user_id = $1 AND used_at IS NULL
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn increment_attempts(pool: &PgPool, token_id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE password_reset_tokens SET attempts = attempts + 1 WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_token_used(pool: &PgPool, token_id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_all_user_refresh_tokens(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    sqlx::query("UPDATE auth_refresh_tokens SET is_revoked = TRUE WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
