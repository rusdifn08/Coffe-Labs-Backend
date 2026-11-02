use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::{
        auth::models::{UserRow, UserSummary},
        users::models::{AdminUserStats, BaristaProfile},
    },
};

const USER_COLS: &str = "id, username, password_hash, full_name, role, status, tier, avatar_url, created_at, updated_at";

pub async fn list_all_users(pool: &PgPool) -> AppResult<Vec<UserSummary>> {
    let q = format!("SELECT {} FROM users ORDER BY created_at DESC", USER_COLS);
    let rows = sqlx::query_as::<_, UserRow>(&q).fetch_all(pool).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn fetch_user_stats(pool: &PgPool) -> AppResult<AdminUserStats> {
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    let new_registrations_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE created_at >= NOW() - INTERVAL '7 days'",
    )
    .fetch_one(pool)
    .await?;
    let active_users: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = 'approved'")
            .fetch_one(pool)
            .await?;
    let pending_or_declined: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status != 'approved'")
            .fetch_one(pool)
            .await?;

    Ok(AdminUserStats {
        total_users,
        new_registrations_7d,
        active_users,
        pending_or_declined,
    })
}

pub async fn update_user_status(
    pool: &PgPool,
    user_id: Uuid,
    status: &str,
    protect_admin: bool,
) -> AppResult<Option<UserSummary>> {
    let condition = if protect_admin {
        "WHERE id = $2 AND role != 'admin'"
    } else {
        "WHERE id = $2"
    };
    let q = format!(
        "UPDATE users SET status = $1, updated_at = NOW() {} RETURNING {}",
        condition, USER_COLS
    );
    let row = sqlx::query_as::<_, UserRow>(&q)
        .bind(status)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(Into::into))
}

pub async fn delete_non_admin_user(pool: &PgPool, user_id: Uuid) -> AppResult<u64> {
    let res = sqlx::query("DELETE FROM users WHERE id = $1 AND role != 'admin'")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn get_profile(pool: &PgPool, user_id: Uuid) -> AppResult<Option<BaristaProfile>> {
    let row = sqlx::query_as::<_, BaristaProfile>(
        "SELECT id, username, full_name, role, status, tier, avatar_url, bio, equipment_json, created_at, updated_at FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update_profile(
    pool: &PgPool,
    user_id: Uuid,
    full_name: Option<&str>,
    avatar_url: Option<&str>,
    bio: Option<&str>,
    equipment_json: Option<serde_json::Value>,
) -> AppResult<Option<BaristaProfile>> {
    let row = sqlx::query_as::<_, BaristaProfile>(
        r#"UPDATE users
           SET full_name = COALESCE($1, full_name),
               avatar_url = COALESCE($2, avatar_url),
               bio = COALESCE($3, bio),
               equipment_json = COALESCE($4, equipment_json),
               updated_at = NOW()
           WHERE id = $5
           RETURNING id, username, full_name, role, status, tier, avatar_url, bio, equipment_json, created_at, updated_at"#,
    )
    .bind(full_name)
    .bind(avatar_url)
    .bind(bio)
    .bind(equipment_json)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
