use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::{
        auth::models::{UserRow, UserSummary},
        users::models::{AdminUserStats, BaristaProfile},
    },
};

const USER_COLS: &str = "id, username, email, password_hash, full_name, role, status, tier, avatar_url, trial_full_until, trial_policy_override, created_at, updated_at";

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

pub async fn update_user_tier(
    pool: &PgPool,
    user_id: Uuid,
    tier: &str,
    trial_days: Option<i32>,
) -> AppResult<Option<UserSummary>> {
    let normalized = tier.trim().to_lowercase();
    let is_trial_7d = normalized == "trial_7d" || normalized == "full_trial_7d";
    let clean_tier = match normalized.as_str() {
        "business" => "business",
        "pro" => "pro",
        _ => "trial",
    };
    let plan_id = match clean_tier {
        "business" => "plan_business",
        "pro" => "plan_pro",
        _ => "plan_trial",
    };
    let days = trial_days.unwrap_or(7).clamp(1, 365);

    let row = if is_trial_7d {
        let q = format!(
            "UPDATE users
             SET tier = 'trial',
                 subscription_plan_id = 'plan_trial',
                 trial_full_until = NOW() + make_interval(days => $1),
                 trial_policy_override = 'full_trial_7d_non_business',
                 updated_at = NOW()
             WHERE id = $2
             RETURNING {}",
            USER_COLS
        );
        sqlx::query_as::<_, UserRow>(&q)
            .bind(days)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
    } else if clean_tier == "trial" {
        let q = format!(
            "UPDATE users
             SET tier = 'trial',
                 subscription_plan_id = 'plan_trial',
                 trial_full_until = NULL,
                 trial_policy_override = 'strict_limited',
                 updated_at = NOW()
             WHERE id = $1
             RETURNING {}",
            USER_COLS
        );
        sqlx::query_as::<_, UserRow>(&q)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
    } else {
        let q = format!(
            "UPDATE users
             SET tier = $1,
                 subscription_plan_id = $2,
                 trial_full_until = NULL,
                 trial_policy_override = NULL,
                 updated_at = NOW()
             WHERE id = $3
             RETURNING {}",
            USER_COLS
        );
        sqlx::query_as::<_, UserRow>(&q)
            .bind(clean_tier)
            .bind(plan_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
    };

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
        "SELECT id, username, email, full_name, role, status, tier, avatar_url, bio, equipment_json, created_at, updated_at FROM users WHERE id = $1",
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
    email: Option<&str>,
    avatar_url: Option<&str>,
    bio: Option<&str>,
    equipment_json: Option<serde_json::Value>,
) -> AppResult<Option<BaristaProfile>> {
    let row = sqlx::query_as::<_, BaristaProfile>(
        r#"UPDATE users
           SET full_name = COALESCE($1, full_name),
               email = COALESCE($2, email),
               avatar_url = COALESCE($3, avatar_url),
               bio = COALESCE($4, bio),
               equipment_json = COALESCE($5, equipment_json),
               updated_at = NOW()
           WHERE id = $6
           RETURNING id, username, email, full_name, role, status, tier, avatar_url, bio, equipment_json, created_at, updated_at"#,
    )
    .bind(full_name)
    .bind(email)
    .bind(avatar_url)
    .bind(bio)
    .bind(equipment_json)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
