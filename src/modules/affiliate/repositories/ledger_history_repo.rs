use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::affiliate::models::{AffiliateConversionRow, AffiliateLedgerRow},
};

pub async fn list_recent_conversions(
    pool: &PgPool,
    referrer_id: Uuid,
    limit: i64,
) -> AppResult<Vec<AffiliateConversionRow>> {
    Ok(sqlx::query_as::<_, AffiliateConversionRow>(
        r#"SELECT c.id, u.username AS buyer_username, u.full_name AS buyer_full_name,
                  c.affiliate_code_used, c.plan_code::text AS plan_code,
                  c.plan_base_price_idr, c.buyer_discount_idr,
                  c.commission_bps, c.commission_earned_idr, c.created_at
           FROM affiliate_referral_conversions c
           INNER JOIN users u ON u.id = c.buyer_user_id
           WHERE c.referrer_user_id = $1 ORDER BY c.created_at DESC LIMIT $2"#,
    )
    .bind(referrer_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?)
}

pub async fn list_recent_ledger(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> AppResult<Vec<AffiliateLedgerRow>> {
    Ok(sqlx::query_as::<_, AffiliateLedgerRow>(
        r#"SELECT id, entry_kind::text AS entry_kind,
                  available_delta_idr, locked_delta_idr,
                  available_balance_after_idr, locked_balance_after_idr, description, created_at
           FROM affiliate_ledger_entries WHERE user_id = $1 ORDER BY id DESC LIMIT $2"#,
    )
    .bind(user_id)
    .bind(limit.clamp(1, 100))
    .fetch_all(pool)
    .await?)
}

pub async fn has_shared_device_fingerprint(
    pool: &PgPool,
    buyer_id: Uuid,
    referrer_id: Uuid,
) -> AppResult<bool> {
    let found: Option<bool> = sqlx::query_scalar(
        r#"SELECT EXISTS(
            SELECT 1 FROM user_device_tokens a
            INNER JOIN user_device_tokens b ON a.device_id = b.device_id
            WHERE a.user_id = $1 AND b.user_id = $2
              AND a.device_id IS NOT NULL AND a.device_id <> ''
              AND a.updated_at >= NOW() - INTERVAL '90 days'
        )"#,
    )
    .bind(buyer_id)
    .bind(referrer_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    Ok(found.unwrap_or(false))
}

pub async fn has_prior_referral_purchase(pool: &PgPool, buyer_id: Uuid) -> AppResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM affiliate_referral_conversions WHERE buyer_user_id = $1)",
    )
    .bind(buyer_id)
    .fetch_one(pool)
    .await?)
}
