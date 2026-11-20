use chrono::{DateTime, Utc};
use tracing::info;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::core::AppState;
use crate::modules::affiliate::services::commission_credit_service;
use crate::modules::qris_billing::dto::BridgeIngestResponseDto;
use crate::modules::qris_billing::error::QrisBillingError;
use crate::modules::qris_billing::models::{QrisInvoiceRow, UpgradedUserRow};
use crate::modules::qris_billing::repositories::{
    invoice_repo,
    subscription_repo::{self, NewMutationParams},
};
use crate::modules::qris_billing::services::bridge_security_guard::compute_mutation_fingerprint;
use crate::modules::qris_billing::services::upgrade_notifier::dispatch_upgrade_side_effects;

pub struct IncomingMutationInput {
    pub source_channel: String,
    pub amount_idr: i64,
    pub payer_name: Option<String>,
    pub bank_rrn_ref: Option<String>,
    pub raw_notification_text: String,
    pub received_at: DateTime<Utc>,
}

/// Mesin rekonsiliasi ACID otomatis (Channel A Android Bridge & Channel B Email IMAP Watcher).
pub async fn reconcile_incoming_payment(
    state: &AppState,
    input: IncomingMutationInput,
) -> AppResult<BridgeIngestResponseDto> {
    let fingerprint = compute_mutation_fingerprint(
        input.amount_idr,
        input.bank_rrn_ref.as_deref(),
        input.received_at.timestamp(),
    );
    let mutation_id = Uuid::now_v7();

    let mut tx = state.pool.begin().await?;
    let inserted = subscription_repo::insert_mutation_idempotent_tx(
        &mut tx,
        NewMutationParams {
            id: mutation_id,
            source_channel: input.source_channel.clone(),
            source_fingerprint: fingerprint,
            amount_idr: input.amount_idr,
            payer_name: input.payer_name.clone(),
            bank_rrn_ref: input.bank_rrn_ref.clone(),
            raw_notification_text: input.raw_notification_text.clone(),
            received_at: input.received_at,
        },
    )
    .await?;

    let Some(saved_mutation_id) = inserted else {
        tx.commit().await?;
        return Ok(BridgeIngestResponseDto {
            mutation_id,
            reconciliation_status: "DUPLICATE_IGNORED".to_string(),
            parsed_amount_idr: input.amount_idr,
            matched_invoice_id: None,
            upgraded_tier: None,
        });
    };

    let matched_invoice =
        invoice_repo::lock_matching_pending_invoice_tx(&mut tx, input.amount_idr).await?;
    let Some(invoice) = matched_invoice else {
        tx.commit().await?;
        info!(
            amount_idr = input.amount_idr,
            channel = %input.source_channel,
            "Mutasi Livin' Merchant tercatat sebagai UNMATCHED (menunggu tinjauan admin)"
        );
        return Ok(BridgeIngestResponseDto {
            mutation_id: saved_mutation_id,
            reconciliation_status: "UNMATCHED".to_string(),
            parsed_amount_idr: input.amount_idr,
            matched_invoice_id: None,
            upgraded_tier: None,
        });
    };

    subscription_repo::mark_invoice_and_mutation_paid_tx(
        &mut tx,
        invoice.id,
        saved_mutation_id,
        "MATCHED_AUTO",
    )
    .await?;

    let upgraded = subscription_repo::upgrade_user_subscription_atomic_tx(
        &mut tx,
        invoice.user_id,
        &invoice.target_tier,
        &invoice.plan_code,
        invoice.duration_days,
    )
    .await?;
    let comm_ev = commission_credit_service::credit_affiliate_commission_tx(&mut tx, &invoice).await?;
    tx.commit().await?;

    if let Some(ref ev) = comm_ev {
        commission_credit_service::dispatch_commission_notifications(state, ev, &upgraded);
    }
    dispatch_upgrade_side_effects(state, &invoice, &upgraded, &input.source_channel).await?;

    Ok(BridgeIngestResponseDto {
        mutation_id: saved_mutation_id,
        reconciliation_status: "MATCHED_AUTO".to_string(),
        parsed_amount_idr: input.amount_idr,
        matched_invoice_id: Some(invoice.id),
        upgraded_tier: Some(upgraded.tier),
    })
}

/// Persetujuan 1-klik Admin (Fallback Layer 3 untuk invoice `MANUAL_REVIEW` / `PENDING` / `EXPIRED`).
pub async fn admin_manual_approve_invoice(
    state: &AppState,
    invoice_id: Uuid,
    admin_username: &str,
) -> AppResult<(QrisInvoiceRow, UpgradedUserRow, String)> {
    let mut tx = state.pool.begin().await?;
    let invoice = invoice_repo::lock_invoice_for_manual_approve_tx(&mut tx, invoice_id)
        .await?
        .ok_or(QrisBillingError::InvoiceNotFound)?;

    let mutation_id = Uuid::now_v7();
    let fingerprint = format!("manual_admin:{}:{}", invoice.id, mutation_id);
    let _ = subscription_repo::insert_mutation_idempotent_tx(
        &mut tx,
        NewMutationParams {
            id: mutation_id,
            source_channel: "MANUAL_ADMIN_APPROVAL".to_string(),
            source_fingerprint: fingerprint,
            amount_idr: invoice.total_payable_idr,
            payer_name: Some(format!("Approved by {admin_username}")),
            bank_rrn_ref: Some(invoice.invoice_number.clone()),
            raw_notification_text: format!(
                "Persetujuan manual admin ({admin_username}) untuk invoice {} sebesar Rp{}",
                invoice.invoice_number, invoice.total_payable_idr
            ),
            received_at: Utc::now(),
        },
    )
    .await?;

    subscription_repo::mark_invoice_and_mutation_paid_tx(
        &mut tx,
        invoice.id,
        mutation_id,
        "MATCHED_MANUAL",
    )
    .await?;

    let upgraded = subscription_repo::upgrade_user_subscription_atomic_tx(
        &mut tx,
        invoice.user_id,
        &invoice.target_tier,
        &invoice.plan_code,
        invoice.duration_days,
    )
    .await?;
    let comm_ev = commission_credit_service::credit_affiliate_commission_tx(&mut tx, &invoice).await?;
    tx.commit().await?;

    if let Some(ref ev) = comm_ev {
        commission_credit_service::dispatch_commission_notifications(state, ev, &upgraded);
    }
    let fresh_token =
        dispatch_upgrade_side_effects(state, &invoice, &upgraded, "MANUAL_ADMIN_APPROVAL").await?;
    let updated_invoice = invoice_repo::find_invoice_by_id(&state.pool, invoice.id)
        .await?
        .unwrap_or(invoice);

    Ok((updated_invoice, upgraded, fresh_token))
}
