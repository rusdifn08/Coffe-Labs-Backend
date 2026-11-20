use chrono::Utc;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::{
        affiliate::repositories::wallet_ledger_repo,
        notifications::events::{NotificationCategory, OmnichannelNotifEvent},
        qris_billing::models::{QrisInvoiceRow, UpgradedUserRow},
    },
};

pub struct CreditedCommissionEvent {
    pub referrer_user_id: Uuid,
    pub commission_idr: i64,
    pub affiliate_code: String,
    pub invoice_id: Uuid,
    pub plan_code: String,
}

/// Dipanggil atomik di dalam transaksi SQLx pelunasan QRIS (`reconcile_upgrade_service.rs`).
/// Menjamin 0 double-credit via `idempotency_key UNIQUE` + `SELECT ... FOR UPDATE`.
pub async fn credit_affiliate_commission_tx(
    tx: &mut Transaction<'_, Postgres>,
    invoice: &QrisInvoiceRow,
) -> AppResult<Option<CreditedCommissionEvent>> {
    let Some(referrer_id) = invoice.referrer_user_id else {
        return Ok(None);
    };
    if invoice.is_commission_credited
        || invoice.affiliate_commission_idr <= 0
        || referrer_id == invoice.user_id
    {
        return Ok(None);
    }

    let Some(wallet) = wallet_ledger_repo::lock_wallet_for_update_tx(tx, referrer_id).await? else {
        return Ok(None);
    };

    let idemp_key = format!("comm:inv:{}", invoice.id);
    let new_avail = wallet
        .available_balance_idr
        .saturating_add(invoice.affiliate_commission_idr);
    let code_used = invoice
        .applied_referral_code
        .clone()
        .unwrap_or_else(|| wallet.affiliate_code.clone());
    let comm_pct = (invoice.affiliate_commission_bps / 100).max(1);
    let desc = format!(
        "Komisi afiliasi {comm_pct}% dari paket {} (Invoice {})",
        invoice.plan_code, invoice.invoice_number
    );

    let inserted_ledger = wallet_ledger_repo::insert_ledger_entry_idempotent_tx(
        tx,
        referrer_id,
        &idemp_key,
        "COMMISSION_CREDIT",
        invoice.affiliate_commission_idr,
        0,
        new_avail,
        wallet.locked_withdrawal_idr,
        Some(invoice.id),
        None,
        None,
        &desc,
    )
    .await?;

    if !inserted_ledger {
        return Ok(None);
    }

    let buyer_net = invoice
        .base_amount_idr
        .saturating_sub(invoice.buyer_discount_idr)
        .max(0);

    sqlx::query(
        r#"INSERT INTO affiliate_referral_conversions (
            id, referrer_user_id, buyer_user_id, source_invoice_id,
            affiliate_code_used, plan_code, plan_base_price_idr,
            buyer_discount_idr, buyer_net_paid_idr, commission_bps, commission_earned_idr
           ) VALUES (
            $1, $2, $3, $4, $5, $6::subscription_plan_code, $7, $8, $9, $10, $11
           ) ON CONFLICT (source_invoice_id) DO NOTHING"#,
    )
    .bind(Uuid::now_v7())
    .bind(referrer_id)
    .bind(invoice.user_id)
    .bind(invoice.id)
    .bind(&code_used)
    .bind(&invoice.plan_code)
    .bind(invoice.base_amount_idr)
    .bind(invoice.buyer_discount_idr)
    .bind(buyer_net)
    .bind(invoice.affiliate_commission_bps)
    .bind(invoice.affiliate_commission_idr)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"UPDATE affiliate_wallets
           SET available_balance_idr = available_balance_idr + $1,
               total_earned_idr = total_earned_idr + $1,
               total_successful_referrals = total_successful_referrals + 1,
               total_code_uses_count = total_code_uses_count + 1,
               updated_at = NOW()
           WHERE user_id = $2"#,
    )
    .bind(invoice.affiliate_commission_idr)
    .bind(referrer_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE qris_subscription_invoices SET is_commission_credited = TRUE, updated_at = NOW() WHERE id = $1",
    )
    .bind(invoice.id)
    .execute(&mut **tx)
    .await?;

    Ok(Some(CreditedCommissionEvent {
        referrer_user_id: referrer_id,
        commission_idr: invoice.affiliate_commission_idr,
        affiliate_code: code_used,
        invoice_id: invoice.id,
        plan_code: invoice.plan_code.clone(),
    }))
}

/// Efek pasca-commit: publish ke RabbitMQ & kirim Push Notification FCM instan ke HP Afiliator.
pub fn dispatch_commission_notifications(
    state: &AppState,
    ev: &CreditedCommissionEvent,
    buyer: &UpgradedUserRow,
) {
    state.rabbitmq.publish_json(
        "",
        "affiliate.commission.earned",
        &json!({
            "event": "AffiliateCommissionEarnedEvent",
            "referrer_user_id": ev.referrer_user_id,
            "buyer_username": buyer.username,
            "affiliate_code": ev.affiliate_code,
            "commission_idr": ev.commission_idr,
            "plan_code": ev.plan_code,
            "invoice_id": ev.invoice_id,
        }),
    );

    let now = Utc::now();
    let tier_label = if ev.plan_code.contains("BUSINESS") {
        "Coffee Labs Business"
    } else {
        "Coffee Labs Pro"
    };
    state.notifications.publish(OmnichannelNotifEvent {
        event_id: Uuid::now_v7(),
        target_user_id: ev.referrer_user_id,
        category: NotificationCategory::Billing,
        event_type: "AFFILIATE_COMMISSION_EARNED".to_string(),
        title: format!("Komisi Masuk +Rp{}!", ev.commission_idr),
        body: format!(
            "@{} baru saja upgrade {} menggunakan kode {} Anda. Saldo siap ditarik!",
            buyer.username, tier_label, ev.affiliate_code
        ),
        deep_link_path: "/affiliate".to_string(),
        coalesce_key: None,
        collapse_key: Some(format!("aff_comm_{}", ev.invoice_id)),
        priority_high: true,
        ttl_secs: NotificationCategory::Billing.default_ttl_secs(),
        created_at: now,
        actor: None,
        actors_preview: Vec::new(),
        aggregated_count: 1,
        entity_id: Some(ev.invoice_id.to_string()),
        conversation_id: None,
        action_buttons: Vec::new(),
        is_critical_transactional: true,
        attempt: 0,
        first_enqueued_at: now,
    });
}
