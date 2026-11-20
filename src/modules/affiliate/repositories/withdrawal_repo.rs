use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::affiliate::models::{AdminWithdrawalQueueRow, AffiliateWithdrawalRow},
};

const WD_COLS: &str = r#"
    id, ticket_number, user_id, requested_amount_idr, admin_fee_idr,
    net_transfer_amount_idr, payout_channel::text AS payout_channel,
    bank_or_wallet_label, account_number, account_holder_name, user_notes,
    status::text AS status, sla_deadline_at, reviewed_by_admin_id,
    admin_transfer_ref_code, transfer_proof_cloudinary_url, admin_notes,
    rejection_reason, processed_at, completed_or_rejected_at, created_at
"#;

pub struct NewWithdrawalParams {
    pub id: Uuid,
    pub ticket_number: String,
    pub user_id: Uuid,
    pub requested_amount_idr: i64,
    pub admin_fee_idr: i64,
    pub net_transfer_amount_idr: i64,
    pub payout_channel: String,
    pub bank_or_wallet_label: String,
    pub account_number: String,
    pub account_holder_name: String,
    pub user_notes: Option<String>,
    pub sla_deadline_at: DateTime<Utc>,
}

pub async fn insert_withdrawal_tx(
    tx: &mut Transaction<'_, Postgres>,
    p: NewWithdrawalParams,
) -> AppResult<AffiliateWithdrawalRow> {
    let sql = format!(
        "INSERT INTO affiliate_withdrawal_requests (
            id, ticket_number, user_id, requested_amount_idr, admin_fee_idr,
            net_transfer_amount_idr, payout_channel, bank_or_wallet_label,
            account_number, account_holder_name, user_notes, status, sla_deadline_at
         ) VALUES (
            $1, $2, $3, $4, $5, $6, $7::payout_channel_kind, $8,
            $9, $10, $11, 'PENDING_ADMIN_REVIEW'::affiliate_withdrawal_status, $12
         ) RETURNING {WD_COLS}"
    );
    Ok(sqlx::query_as::<_, AffiliateWithdrawalRow>(&sql)
        .bind(p.id)
        .bind(p.ticket_number)
        .bind(p.user_id)
        .bind(p.requested_amount_idr)
        .bind(p.admin_fee_idr)
        .bind(p.net_transfer_amount_idr)
        .bind(p.payout_channel)
        .bind(p.bank_or_wallet_label)
        .bind(p.account_number)
        .bind(p.account_holder_name)
        .bind(p.user_notes)
        .bind(p.sla_deadline_at)
        .fetch_one(&mut **tx)
        .await?)
}

pub async fn lock_withdrawal_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    withdrawal_id: Uuid,
) -> AppResult<Option<AffiliateWithdrawalRow>> {
    let sql = format!(
        "SELECT {WD_COLS} FROM affiliate_withdrawal_requests WHERE id = $1 FOR UPDATE"
    );
    Ok(sqlx::query_as::<_, AffiliateWithdrawalRow>(&sql)
        .bind(withdrawal_id)
        .fetch_optional(&mut **tx)
        .await?)
}

pub async fn list_user_withdrawals(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> AppResult<Vec<AffiliateWithdrawalRow>> {
    let sql = format!(
        "SELECT {WD_COLS} FROM affiliate_withdrawal_requests
         WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2"
    );
    Ok(sqlx::query_as::<_, AffiliateWithdrawalRow>(&sql)
        .bind(user_id)
        .bind(limit.clamp(1, 100))
        .fetch_all(pool)
        .await?)
}

pub async fn list_admin_withdrawal_queue(
    pool: &PgPool,
    status_filter: Option<&str>,
    limit: i64,
) -> AppResult<Vec<AdminWithdrawalQueueRow>> {
    let rows = sqlx::query_as::<_, AdminWithdrawalQueueRow>(
        r#"SELECT w.id, w.ticket_number, w.user_id,
                  u.username AS affiliate_username, u.full_name AS affiliate_full_name,
                  COALESCE(aw.affiliate_code, '') AS affiliate_code,
                  w.requested_amount_idr, w.admin_fee_idr, w.net_transfer_amount_idr,
                  w.payout_channel::text AS payout_channel, w.bank_or_wallet_label,
                  w.account_number, w.account_holder_name, w.user_notes,
                  w.status::text AS status, w.sla_deadline_at,
                  w.admin_transfer_ref_code, w.transfer_proof_cloudinary_url,
                  w.rejection_reason, w.created_at, w.completed_or_rejected_at
           FROM affiliate_withdrawal_requests w
           INNER JOIN users u ON u.id = w.user_id
           LEFT JOIN affiliate_wallets aw ON aw.user_id = w.user_id
           WHERE ($1::text IS NULL OR w.status::text = $1)
           ORDER BY
               CASE w.status::text
                   WHEN 'PENDING_ADMIN_REVIEW' THEN 1
                   WHEN 'PROCESSING_BY_ADMIN' THEN 2
                   ELSE 3
               END ASC,
               w.created_at DESC
           LIMIT $2"#,
    )
    .bind(status_filter)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_admin_user_ids(pool: &PgPool) -> AppResult<Vec<Uuid>> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(role) = 'admin' LIMIT 10",
    )
    .fetch_all(pool)
    .await?;
    Ok(ids)
}
