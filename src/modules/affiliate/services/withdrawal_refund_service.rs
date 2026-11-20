use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::affiliate::{
        dto::AdminRejectWithdrawalDto,
        error::AffiliateError,
        models::AffiliateWithdrawalRow,
        repositories::{wallet_ledger_repo, withdrawal_repo},
        services::affiliate_notifier::push_user_payout_notif,
    },
};

const WD_RET: &str = r#"
    id, ticket_number, user_id, requested_amount_idr, admin_fee_idr,
    net_transfer_amount_idr, payout_channel::text AS payout_channel,
    bank_or_wallet_label, account_number, account_holder_name, user_notes,
    status::text AS status, sla_deadline_at, reviewed_by_admin_id,
    admin_transfer_ref_code, transfer_proof_cloudinary_url, admin_notes,
    rejection_reason, processed_at, completed_or_rejected_at, created_at
"#;

pub async fn reject_or_cancel_withdrawal(
    state: &AppState,
    withdrawal_id: Uuid,
    actor_user_id: Uuid,
    is_admin_reject: bool,
    reject_req: Option<AdminRejectWithdrawalDto>,
) -> AppResult<AffiliateWithdrawalRow> {
    let reason = if is_admin_reject {
        let r = reject_req.map(|d| d.rejection_reason.trim().to_string()).unwrap_or_default();
        if r.len() < 4 {
            return Err(AffiliateError::InvalidPayoutDetails(
                "Alasan penolakan wajib diisi minimal 4 karakter.".into(),
            )
            .into());
        }
        r
    } else {
        "Dibatalkan sendiri oleh afiliator sebelum diproses.".to_string()
    };

    let mut tx = state.pool.begin().await?;
    let wd = withdrawal_repo::lock_withdrawal_for_update_tx(&mut tx, withdrawal_id)
        .await?
        .ok_or(AffiliateError::WithdrawalNotFound)?;

    if !is_admin_reject && (wd.user_id != actor_user_id || wd.status != "PENDING_ADMIN_REVIEW") {
        return Err(AffiliateError::InvalidWithdrawalTransition(
            "Penarikan hanya dapat dibatalkan oleh pemilik saat status masih Menunggu ACC.".into(),
        )
        .into());
    }
    if is_admin_reject && wd.status != "PENDING_ADMIN_REVIEW" && wd.status != "PROCESSING_BY_ADMIN" {
        return Err(AffiliateError::InvalidWithdrawalTransition("Tiket ini sudah diproses final.".into()).into());
    }

    let wallet = wallet_ledger_repo::lock_wallet_for_update_tx(&mut tx, wd.user_id)
        .await?
        .ok_or(AffiliateError::WithdrawalNotFound)?;
    let new_avail = wallet.available_balance_idr + wd.requested_amount_idr;
    let new_locked = wallet.locked_withdrawal_idr.saturating_sub(wd.requested_amount_idr);
    let next_status = if is_admin_reject { "REJECTED_REFUNDED" } else { "CANCELLED_BY_USER" };
    let ledger_kind = if is_admin_reject { "WITHDRAWAL_REFUND_REJECTED" } else { "WITHDRAWAL_REFUND_CANCELLED" };

    sqlx::query("UPDATE affiliate_wallets SET available_balance_idr = $1, locked_withdrawal_idr = $2, updated_at = NOW() WHERE user_id = $3")
        .bind(new_avail)
        .bind(new_locked)
        .bind(wd.user_id)
        .execute(&mut *tx)
        .await?;

    let sql = format!(
        "UPDATE affiliate_withdrawal_requests
         SET status = $1::affiliate_withdrawal_status,
             reviewed_by_admin_id = COALESCE($2, reviewed_by_admin_id),
             rejection_reason = $3, completed_or_rejected_at = NOW(), updated_at = NOW()
         WHERE id = $4 RETURNING {WD_RET}"
    );
    let updated = sqlx::query_as::<_, AffiliateWithdrawalRow>(&sql)
        .bind(next_status)
        .bind(is_admin_reject.then_some(actor_user_id))
        .bind(&reason)
        .bind(withdrawal_id)
        .fetch_one(&mut *tx)
        .await?;

    wallet_ledger_repo::insert_ledger_entry_idempotent_tx(
        &mut tx,
        wd.user_id,
        &format!("wd:refund:{withdrawal_id}"),
        ledger_kind,
        wd.requested_amount_idr,
        -wd.requested_amount_idr,
        new_avail,
        new_locked,
        None,
        Some(withdrawal_id),
        is_admin_reject.then_some(actor_user_id),
        &format!("Pengembalian dana {} (Rp{}): {reason}", wd.ticket_number, wd.requested_amount_idr),
    )
    .await?;

    tx.commit().await?;
    if is_admin_reject {
        push_user_payout_notif(
            state,
            wd.user_id,
            &format!("Penarikan {} Ditolak — Saldo Dikembalikan", wd.ticket_number),
            &format!("Dana Rp{} dikembalikan penuh ke saldo afiliasi Anda. Alasan: {reason}", wd.requested_amount_idr),
            withdrawal_id,
        );
    }
    Ok(updated)
}
