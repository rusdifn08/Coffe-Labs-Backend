use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::affiliate::{
        dto::AdminApproveWithdrawalDto,
        error::AffiliateError,
        models::AffiliateWithdrawalRow,
        repositories::{wallet_ledger_repo, withdrawal_repo},
        services::affiliate_notifier::push_user_payout_notif,
    },
};

pub use crate::modules::affiliate::services::withdrawal_refund_service::reject_or_cancel_withdrawal;

const WD_RET: &str = r#"
    id, ticket_number, user_id, requested_amount_idr, admin_fee_idr,
    net_transfer_amount_idr, payout_channel::text AS payout_channel,
    bank_or_wallet_label, account_number, account_holder_name, user_notes,
    status::text AS status, sla_deadline_at, reviewed_by_admin_id,
    admin_transfer_ref_code, transfer_proof_cloudinary_url, admin_notes,
    rejection_reason, processed_at, completed_or_rejected_at, created_at
"#;

pub async fn mark_withdrawal_processing(
    state: &AppState,
    withdrawal_id: Uuid,
    admin_id: Uuid,
) -> AppResult<AffiliateWithdrawalRow> {
    let mut tx = state.pool.begin().await?;
    let wd = withdrawal_repo::lock_withdrawal_for_update_tx(&mut tx, withdrawal_id)
        .await?
        .ok_or(AffiliateError::WithdrawalNotFound)?;
    if wd.status != "PENDING_ADMIN_REVIEW" {
        return Err(AffiliateError::InvalidWithdrawalTransition(
            "Hanya tiket PENDING_ADMIN_REVIEW yang dapat ditandai sedang diproses.".into(),
        )
        .into());
    }
    let sql = format!(
        "UPDATE affiliate_withdrawal_requests
         SET status = 'PROCESSING_BY_ADMIN'::affiliate_withdrawal_status,
             reviewed_by_admin_id = $1, processed_at = NOW(), updated_at = NOW()
         WHERE id = $2 RETURNING {WD_RET}"
    );
    let updated = sqlx::query_as::<_, AffiliateWithdrawalRow>(&sql)
        .bind(admin_id)
        .bind(withdrawal_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(updated)
}

pub async fn approve_withdrawal(
    state: &AppState,
    withdrawal_id: Uuid,
    admin_id: Uuid,
    req: AdminApproveWithdrawalDto,
) -> AppResult<AffiliateWithdrawalRow> {
    let proof_url = req.transfer_proof_cloudinary_url.trim();
    if !proof_url.starts_with("http://") && !proof_url.starts_with("https://") {
        return Err(AffiliateError::InvalidPayoutDetails(
            "URL bukti transfer Cloudinary wajib valid (https://...).".into(),
        )
        .into());
    }

    let mut tx = state.pool.begin().await?;
    let wd = withdrawal_repo::lock_withdrawal_for_update_tx(&mut tx, withdrawal_id)
        .await?
        .ok_or(AffiliateError::WithdrawalNotFound)?;
    if wd.status != "PENDING_ADMIN_REVIEW" && wd.status != "PROCESSING_BY_ADMIN" {
        return Err(AffiliateError::InvalidWithdrawalTransition(
            "Tiket ini sudah selesai atau dibatalkan sebelumnya.".into(),
        )
        .into());
    }
    let wallet = wallet_ledger_repo::lock_wallet_for_update_tx(&mut tx, wd.user_id)
        .await?
        .ok_or(AffiliateError::WithdrawalNotFound)?;
    let new_locked = wallet.locked_withdrawal_idr.saturating_sub(wd.requested_amount_idr);

    sqlx::query(
        "UPDATE affiliate_wallets SET locked_withdrawal_idr = $1, total_withdrawn_idr = total_withdrawn_idr + $2, updated_at = NOW() WHERE user_id = $3",
    )
    .bind(new_locked)
    .bind(wd.requested_amount_idr)
    .bind(wd.user_id)
    .execute(&mut *tx)
    .await?;

    let sql = format!(
        "UPDATE affiliate_withdrawal_requests
         SET status = 'COMPLETED_TRANSFERRED'::affiliate_withdrawal_status,
             reviewed_by_admin_id = $1, transfer_proof_cloudinary_url = $2,
             transfer_proof_cloudinary_public_id = $3, admin_transfer_ref_code = $4,
             admin_notes = $5, completed_or_rejected_at = NOW(), updated_at = NOW()
         WHERE id = $6 RETURNING {WD_RET}"
    );
    let updated = sqlx::query_as::<_, AffiliateWithdrawalRow>(&sql)
        .bind(admin_id)
        .bind(proof_url)
        .bind(req.transfer_proof_cloudinary_public_id.as_deref())
        .bind(req.admin_transfer_ref_code.as_deref())
        .bind(req.admin_notes.as_deref())
        .bind(withdrawal_id)
        .fetch_one(&mut *tx)
        .await?;

    wallet_ledger_repo::insert_ledger_entry_idempotent_tx(
        &mut tx,
        wd.user_id,
        &format!("wd:settled:{withdrawal_id}"),
        "WITHDRAWAL_SETTLED",
        0,
        -wd.requested_amount_idr,
        wallet.available_balance_idr,
        new_locked,
        None,
        Some(withdrawal_id),
        Some(admin_id),
        &format!("Penarikan {} (Rp{}) berhasil ditransfer ke {}", wd.ticket_number, wd.net_transfer_amount_idr, wd.bank_or_wallet_label),
    )
    .await?;

    tx.commit().await?;
    push_user_payout_notif(
        state,
        wd.user_id,
        &format!("Penarikan Rp{} Berhasil!", wd.net_transfer_amount_idr),
        &format!("Dana tiket {} telah ditransfer ke {} ({}). Lihat bukti transfer di riwayat penarikan.", wd.ticket_number, wd.bank_or_wallet_label, wd.account_number),
        withdrawal_id,
    );
    Ok(updated)
}
