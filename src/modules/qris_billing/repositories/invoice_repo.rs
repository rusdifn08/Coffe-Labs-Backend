use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::modules::qris_billing::models::{QrisInvoiceRow, QrisMutationRow, SubscriptionPlanRow};

const INVOICE_COLS: &str = r#"
    id, invoice_number, user_id, plan_code::text AS plan_code,
    target_tier, duration_days, base_amount_idr, unique_code_idr,
    total_payable_idr, dynamic_qris_payload, merchant_name, acquirer_code,
    status::text AS status, expires_at, paid_at, matched_mutation_id,
    manual_proof_cloudinary_url, receipt_pdf_cloudinary_url,
    applied_referral_code, referrer_user_id, buyer_discount_idr,
    affiliate_commission_bps, affiliate_commission_idr, is_commission_credited,
    created_at, updated_at
"#;

pub async fn list_active_plans(pool: &PgPool) -> AppResult<Vec<SubscriptionPlanRow>> {
    Ok(sqlx::query_as::<_, SubscriptionPlanRow>(
        "SELECT plan_code::text AS plan_code, target_tier, display_name,
                duration_days, base_price_idr, original_price_idr,
                promo_badge_text, COALESCE(is_highlighted, FALSE) AS is_highlighted,
                features_json, is_active
         FROM subscription_plan_catalog WHERE is_active = TRUE ORDER BY base_price_idr ASC",
    ).fetch_all(pool).await?)
}

pub async fn find_plan_by_code(pool: &PgPool, plan_code: &str) -> AppResult<Option<SubscriptionPlanRow>> {
    Ok(sqlx::query_as::<_, SubscriptionPlanRow>(
        "SELECT plan_code::text AS plan_code, target_tier, display_name,
                duration_days, base_price_idr, original_price_idr,
                promo_badge_text, COALESCE(is_highlighted, FALSE) AS is_highlighted,
                features_json, is_active
         FROM subscription_plan_catalog WHERE plan_code::text = $1 AND is_active = TRUE",
    ).bind(plan_code).fetch_optional(pool).await?)
}

pub async fn find_active_pending_for_user_plan(
    pool: &PgPool,
    user_id: Uuid,
    plan_code: &str,
) -> AppResult<Option<QrisInvoiceRow>> {
    let sql = format!(
        "SELECT {INVOICE_COLS} FROM qris_subscription_invoices
         WHERE user_id = $1 AND plan_code::text = $2
           AND status = 'PENDING'::qris_invoice_status
           AND expires_at > NOW() + INTERVAL '60 seconds'
         ORDER BY created_at DESC LIMIT 1"
    );
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql).bind(user_id).bind(plan_code).fetch_optional(pool).await?)
}

pub struct NewInvoiceParams {
    pub id: Uuid,
    pub invoice_number: String,
    pub user_id: Uuid,
    pub plan_code: String,
    pub target_tier: String,
    pub duration_days: i32,
    pub base_amount_idr: i64,
    pub unique_code_idr: i16,
    pub total_payable_idr: i64,
    pub dynamic_qris_payload: String,
    pub merchant_name: String,
    pub acquirer_code: String,
    pub expires_at: DateTime<Utc>,
    pub applied_referral_code: Option<String>,
    pub referrer_user_id: Option<Uuid>,
    pub buyer_discount_idr: i64,
    pub affiliate_commission_bps: i32,
    pub affiliate_commission_idr: i64,
}

pub async fn insert_invoice(pool: &PgPool, p: NewInvoiceParams) -> AppResult<QrisInvoiceRow> {
    let sql = format!(
        "INSERT INTO qris_subscription_invoices (
            id, invoice_number, user_id, plan_code, target_tier, duration_days,
            base_amount_idr, unique_code_idr, total_payable_idr,
            dynamic_qris_payload, merchant_name, acquirer_code, status, expires_at,
            applied_referral_code, referrer_user_id, buyer_discount_idr,
            affiliate_commission_bps, affiliate_commission_idr
         ) VALUES (
            $1, $2, $3, $4::subscription_plan_code, $5, $6,
            $7, $8, $9, $10, $11, $12, 'PENDING'::qris_invoice_status, $13,
            $14, $15, $16, $17, $18
         ) RETURNING {INVOICE_COLS}"
    );
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql)
        .bind(p.id).bind(p.invoice_number).bind(p.user_id).bind(p.plan_code)
        .bind(p.target_tier).bind(p.duration_days).bind(p.base_amount_idr)
        .bind(p.unique_code_idr).bind(p.total_payable_idr).bind(p.dynamic_qris_payload)
        .bind(p.merchant_name).bind(p.acquirer_code).bind(p.expires_at)
        .bind(p.applied_referral_code).bind(p.referrer_user_id).bind(p.buyer_discount_idr)
        .bind(p.affiliate_commission_bps).bind(p.affiliate_commission_idr)
        .fetch_one(pool)
        .await?)
}

pub async fn find_invoice_by_id(pool: &PgPool, id: Uuid) -> AppResult<Option<QrisInvoiceRow>> {
    let sql = format!("SELECT {INVOICE_COLS} FROM qris_subscription_invoices WHERE id = $1");
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql).bind(id).fetch_optional(pool).await?)
}

pub async fn attach_manual_proof(
    pool: &PgPool,
    invoice_id: Uuid,
    user_id: Uuid,
    proof_url: &str,
) -> AppResult<Option<QrisInvoiceRow>> {
    let sql = format!(
        "UPDATE qris_subscription_invoices
         SET manual_proof_cloudinary_url = $1,
             status = CASE WHEN status = 'PAID'::qris_invoice_status THEN status ELSE 'MANUAL_REVIEW'::qris_invoice_status END,
             updated_at = NOW()
         WHERE id = $2 AND user_id = $3
         RETURNING {INVOICE_COLS}"
    );
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql)
        .bind(proof_url)
        .bind(invoice_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?)
}

pub async fn expire_stale_pending_invoices(pool: &PgPool) -> AppResult<Vec<(Uuid, i64)>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "UPDATE qris_subscription_invoices
         SET status = 'EXPIRED'::qris_invoice_status, updated_at = NOW()
         WHERE status = 'PENDING'::qris_invoice_status AND expires_at <= NOW()
         RETURNING id, total_payable_idr",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn lock_matching_pending_invoice_tx(
    tx: &mut Transaction<'_, Postgres>,
    amount_idr: i64,
) -> AppResult<Option<QrisInvoiceRow>> {
    let sql = format!(
        "SELECT {INVOICE_COLS} FROM qris_subscription_invoices
         WHERE total_payable_idr = $1
           AND status = 'PENDING'::qris_invoice_status
           AND expires_at >= NOW() - INTERVAL '2 minutes'
         ORDER BY created_at DESC LIMIT 1
         FOR UPDATE SKIP LOCKED"
    );
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql).bind(amount_idr).fetch_optional(&mut **tx).await?)
}

pub async fn lock_invoice_for_manual_approve_tx(
    tx: &mut Transaction<'_, Postgres>,
    invoice_id: Uuid,
) -> AppResult<Option<QrisInvoiceRow>> {
    let sql = format!(
        "SELECT {INVOICE_COLS} FROM qris_subscription_invoices
         WHERE id = $1 AND status IN ('PENDING'::qris_invoice_status, 'MANUAL_REVIEW'::qris_invoice_status, 'EXPIRED'::qris_invoice_status)
         FOR UPDATE"
    );
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql).bind(invoice_id).fetch_optional(&mut **tx).await?)
}

pub async fn list_recent_mutations(pool: &PgPool, limit: i64) -> AppResult<Vec<QrisMutationRow>> {
    let rows = sqlx::query_as::<_, QrisMutationRow>(
        r#"SELECT id, source_channel, source_fingerprint, amount_idr, payer_name,
                  bank_rrn_ref, raw_notification_text, received_at,
                  matched_invoice_id, reconciliation_status, created_at
           FROM qris_payment_mutations
           ORDER BY received_at DESC LIMIT $1"#,
    )
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_manual_review_invoices(pool: &PgPool) -> AppResult<Vec<QrisInvoiceRow>> {
    let sql = format!(
        "SELECT {INVOICE_COLS} FROM qris_subscription_invoices
         WHERE status IN ('MANUAL_REVIEW'::qris_invoice_status, 'PENDING'::qris_invoice_status)
         ORDER BY created_at DESC LIMIT 50"
    );
    Ok(sqlx::query_as::<_, QrisInvoiceRow>(&sql).fetch_all(pool).await?)
}
