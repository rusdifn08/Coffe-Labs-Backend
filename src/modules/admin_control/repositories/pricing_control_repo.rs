use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::admin_control::models::{AuditLogRow, DynamicPlanRow},
};

const PLAN_COLS: &str = r#"
    plan_code::text AS plan_code, target_tier, display_name, duration_days,
    base_price_idr, original_price_idr, promo_badge_text,
    COALESCE(is_highlighted, FALSE) AS is_highlighted,
    features_json, is_active, updated_at
"#;

pub async fn list_all_plans(pool: &PgPool) -> AppResult<Vec<DynamicPlanRow>> {
    let sql = format!(
        "SELECT {PLAN_COLS} FROM subscription_plan_catalog
         ORDER BY is_active DESC, target_tier ASC, duration_days DESC, base_price_idr ASC"
    );
    Ok(sqlx::query_as::<_, DynamicPlanRow>(&sql).fetch_all(pool).await?)
}

pub async fn find_plan_by_code_tx(
    tx: &mut Transaction<'_, Postgres>,
    plan_code: &str,
) -> AppResult<Option<DynamicPlanRow>> {
    let sql = format!(
        "SELECT {PLAN_COLS} FROM subscription_plan_catalog
         WHERE plan_code::text = $1 FOR UPDATE"
    );
    Ok(sqlx::query_as::<_, DynamicPlanRow>(&sql)
        .bind(plan_code)
        .fetch_optional(&mut **tx)
        .await?)
}

pub struct UpdatePlanParams<'a> {
    pub plan_code: &'a str,
    pub display_name: &'a str,
    pub base_price_idr: i64,
    pub original_price_idr: Option<i64>,
    pub promo_badge_text: Option<&'a str>,
    pub duration_days: i32,
    pub is_active: bool,
    pub is_highlighted: bool,
    pub admin_user_id: Uuid,
}

pub async fn update_plan_pricing_tx(
    tx: &mut Transaction<'_, Postgres>,
    p: UpdatePlanParams<'_>,
) -> AppResult<DynamicPlanRow> {
    let sql = format!(
        "UPDATE subscription_plan_catalog
         SET display_name = $2,
             base_price_idr = $3,
             original_price_idr = $4,
             promo_badge_text = $5,
             duration_days = $6,
             is_active = $7,
             is_highlighted = $8,
             updated_by_admin_id = $9,
             updated_at = NOW()
         WHERE plan_code::text = $1
         RETURNING {PLAN_COLS}"
    );
    Ok(sqlx::query_as::<_, DynamicPlanRow>(&sql)
        .bind(p.plan_code)
        .bind(p.display_name)
        .bind(p.base_price_idr)
        .bind(p.original_price_idr)
        .bind(p.promo_badge_text)
        .bind(p.duration_days)
        .bind(p.is_active)
        .bind(p.is_highlighted)
        .bind(p.admin_user_id)
        .fetch_one(&mut **tx)
        .await?)
}

pub struct NewAuditLogParams<'a> {
    pub admin_user_id: Uuid,
    pub admin_email: &'a str,
    pub action_category: &'a str,
    pub target_key: &'a str,
    pub previous_state: Value,
    pub new_state: Value,
    pub reason_note: Option<&'a str>,
    pub config_version_after: i64,
}

pub async fn insert_audit_log_tx(
    tx: &mut Transaction<'_, Postgres>,
    p: NewAuditLogParams<'_>,
) -> AppResult<()> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO admin_control_audit_logs (
            id, admin_user_id, admin_email, action_category, target_key,
            previous_state_json, new_state_json, reason_note, config_version_after
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(p.admin_user_id)
    .bind(p.admin_email)
    .bind(p.action_category)
    .bind(p.target_key)
    .bind(p.previous_state)
    .bind(p.new_state)
    .bind(p.reason_note)
    .bind(p.config_version_after)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn list_recent_audit_logs(pool: &PgPool, limit: i64) -> AppResult<Vec<AuditLogRow>> {
    Ok(sqlx::query_as::<_, AuditLogRow>(
        "SELECT id, admin_user_id, admin_email, action_category, target_key,
                previous_state_json, new_state_json, reason_note,
                config_version_after, created_at
         FROM admin_control_audit_logs
         ORDER BY created_at DESC, id DESC
         LIMIT $1",
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?)
}

pub async fn resolve_admin_email(pool: &PgPool, admin_user_id: Uuid, fallback: &str) -> String {
    sqlx::query_scalar::<_, String>(
        "SELECT COALESCE(NULLIF(TRIM(email), ''), username) FROM users WHERE id = $1",
    )
    .bind(admin_user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .filter(|s| !s.trim().is_empty())
    .unwrap_or_else(|| fallback.to_string())
}
