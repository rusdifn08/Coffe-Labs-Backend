use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{core::error::AppResult, modules::rbac::models::UserRbacState};

const RBAC_COLS: &str = "id, username, role, tier, trial_brews_count, trial_locked_until, subscription_plan_id, subscription_payment_method, subscription_activated_at";

pub async fn get_user_rbac(pool: &PgPool, user_id: Uuid) -> AppResult<Option<UserRbacState>> {
    let q = format!("SELECT {} FROM users WHERE id = $1", RBAC_COLS);
    let row = sqlx::query_as::<_, UserRbacState>(&q)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn update_user_tier(
    pool: &PgPool,
    user_id: Uuid,
    tier: &str,
    plan_id: &str,
    payment_method: &str,
) -> AppResult<UserRbacState> {
    let q = format!(
        "UPDATE users
         SET tier = $1,
             subscription_plan_id = $2,
             subscription_payment_method = $3,
             subscription_activated_at = NOW(),
             trial_locked_until = NULL,
             updated_at = NOW()
         WHERE id = $4
         RETURNING {}",
        RBAC_COLS
    );
    let row = sqlx::query_as::<_, UserRbacState>(&q)
        .bind(tier)
        .bind(plan_id)
        .bind(payment_method)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row)
}

pub async fn update_trial_counters(
    pool: &PgPool,
    user_id: Uuid,
    count: i32,
    locked_until: Option<DateTime<Utc>>,
) -> AppResult<UserRbacState> {
    let q = format!(
        "UPDATE users
         SET trial_brews_count = $1,
             trial_locked_until = $2,
             updated_at = NOW()
         WHERE id = $3
         RETURNING {}",
        RBAC_COLS
    );
    let row = sqlx::query_as::<_, UserRbacState>(&q)
        .bind(count)
        .bind(locked_until)
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    Ok(row)
}
