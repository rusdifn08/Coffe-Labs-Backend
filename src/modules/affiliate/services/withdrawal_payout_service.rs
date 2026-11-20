use chrono::Utc;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::affiliate::{
        code_generator::{generate_withdrawal_ticket, is_pro_eligible_tier},
        dto::CreateWithdrawalDto,
        error::AffiliateError,
        math_and_sla::compute_business_day_24h_sla,
        models::{AffiliateConfigRow, AffiliateWithdrawalRow},
        repositories::{
            wallet_ledger_repo,
            withdrawal_repo::{self, NewWithdrawalParams},
        },
        services::affiliate_notifier::notify_admins_new_withdrawal,
    },
};

pub use crate::modules::affiliate::services::payout_channel_guard::{
    acquire_wd_lock, normalize_payout_channel, release_wd_lock,
};

/// Pengajuan Tarik Saldo Manual (Minimal Rp 100.000 • SLA 1x24 Jam Kerja • Two-Bucket Hold ACID).
pub async fn request_user_withdrawal(
    state: &AppState,
    user_id: Uuid,
    req: CreateWithdrawalDto,
) -> AppResult<AffiliateWithdrawalRow> {
    let (tier, role, username) = wallet_ledger_repo::fetch_user_tier_and_name(&state.pool, user_id)
        .await?
        .ok_or(AffiliateError::ProAccountRequired)?;
    if !is_pro_eligible_tier(&tier, &role) {
        return Err(AffiliateError::ProAccountRequired.into());
    }

    let (channel_db, default_label) = normalize_payout_channel(&req.payout_channel)
        .ok_or_else(|| AffiliateError::InvalidPayoutDetails("Channel Bank / E-Wallet tidak dikenali.".into()))?;
    let acct_num = req.account_number.trim();
    let acct_holder = req.account_holder_name.trim();
    if acct_num.len() < 6 || acct_num.len() > 40 || acct_holder.len() < 2 {
        return Err(AffiliateError::InvalidPayoutDetails(
            "Nomor rekening/e-wallet (min 6 digit) dan nama pemilik rekening wajib diisi lengkap.".into(),
        )
        .into());
    }

    let cfg = wallet_ledger_repo::fetch_program_config(&state.pool).await?;
    if req.requested_amount_idr < cfg.min_withdrawal_idr {
        return Err(AffiliateError::BelowMinWithdrawal(cfg.min_withdrawal_idr).into());
    }

    let lock_key = acquire_wd_lock(state, user_id).await?;
    let res = execute_hold_tx(state, user_id, &cfg, &req, channel_db, default_label).await;
    release_wd_lock(state, lock_key).await;
    let wd = res?;

    notify_admins_new_withdrawal(state, &username, &wd).await;
    Ok(wd)
}

async fn execute_hold_tx(
    state: &AppState,
    user_id: Uuid,
    cfg: &AffiliateConfigRow,
    req: &CreateWithdrawalDto,
    channel_db: &str,
    default_label: &str,
) -> AppResult<AffiliateWithdrawalRow> {
    let mut tx = state.pool.begin().await?;
    let wallet = wallet_ledger_repo::lock_wallet_for_update_tx(&mut tx, user_id)
        .await?
        .ok_or(AffiliateError::InsufficientBalance {
            available: 0,
            requested: req.requested_amount_idr,
        })?;

    if wallet.available_balance_idr < req.requested_amount_idr {
        return Err(AffiliateError::InsufficientBalance {
            available: wallet.available_balance_idr,
            requested: req.requested_amount_idr,
        }
        .into());
    }

    let now = Utc::now();
    let wd_id = Uuid::now_v7();
    let ticket = generate_withdrawal_ticket(now);
    let net = req.requested_amount_idr.saturating_sub(cfg.withdrawal_admin_fee_idr).max(1);
    let label = req
        .bank_or_wallet_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default_label)
        .to_string();

    let new_avail = wallet.available_balance_idr - req.requested_amount_idr;
    let new_locked = wallet.locked_withdrawal_idr + req.requested_amount_idr;

    let wd = withdrawal_repo::insert_withdrawal_tx(
        &mut tx,
        NewWithdrawalParams {
            id: wd_id,
            ticket_number: ticket.clone(),
            user_id,
            requested_amount_idr: req.requested_amount_idr,
            admin_fee_idr: cfg.withdrawal_admin_fee_idr,
            net_transfer_amount_idr: net,
            payout_channel: channel_db.to_string(),
            bank_or_wallet_label: label.clone(),
            account_number: req.account_number.trim().to_string(),
            account_holder_name: req.account_holder_name.trim().to_string(),
            user_notes: req.user_notes.as_deref().map(str::trim).map(ToOwned::to_owned),
            sla_deadline_at: compute_business_day_24h_sla(now),
        },
    )
    .await?;

    sqlx::query(
        r#"UPDATE affiliate_wallets
           SET available_balance_idr = $1, locked_withdrawal_idr = $2,
               saved_payout_channel = $3::payout_channel_kind,
               saved_account_number = $4, saved_account_holder_name = $5, updated_at = NOW()
           WHERE user_id = $6"#,
    )
    .bind(new_avail).bind(new_locked).bind(channel_db)
    .bind(req.account_number.trim()).bind(req.account_holder_name.trim()).bind(user_id)
    .execute(&mut *tx)
    .await?;

    wallet_ledger_repo::insert_ledger_entry_idempotent_tx(
        &mut tx,
        user_id,
        &format!("wd:hold:{wd_id}"),
        "WITHDRAWAL_HOLD",
        -req.requested_amount_idr,
        req.requested_amount_idr,
        new_avail,
        new_locked,
        None,
        Some(wd_id),
        None,
        &format!("Tarik saldo {ticket} ke {label} ({}) — menunggu ACC Admin", req.account_number.trim()),
    )
    .await?;

    tx.commit().await?;
    Ok(wd)
}
