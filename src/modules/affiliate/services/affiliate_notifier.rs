use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::AppState,
    modules::{
        affiliate::{models::AffiliateWithdrawalRow, repositories::withdrawal_repo},
        notifications::events::{NotificationCategory, OmnichannelNotifEvent},
    },
};

pub async fn notify_admins_new_withdrawal(
    state: &AppState,
    username: &str,
    wd: &AffiliateWithdrawalRow,
) {
    state.rabbitmq.publish_json(
        "",
        "affiliate.withdrawal.requested",
        &json!({
            "event": "AffiliateWithdrawalRequestedEvent",
            "withdrawal_id": wd.id,
            "ticket_number": wd.ticket_number,
            "username": username,
            "amount_idr": wd.net_transfer_amount_idr,
            "channel": wd.bank_or_wallet_label,
        }),
    );

    if let Ok(admin_ids) = withdrawal_repo::list_admin_user_ids(&state.pool).await {
        let now = Utc::now();
        for admin_id in admin_ids {
            state.notifications.publish(OmnichannelNotifEvent {
                event_id: Uuid::now_v7(),
                target_user_id: admin_id,
                category: NotificationCategory::Billing,
                event_type: "ADMIN_AFFILIATE_WITHDRAWAL_PENDING".to_string(),
                title: format!("Tarik Saldo Rp{} ({})", wd.net_transfer_amount_idr, wd.ticket_number),
                body: format!(
                    "@{username} mengajukan tarik ke {} a/n {}. Proses 1x24 jam kerja.",
                    wd.bank_or_wallet_label, wd.account_holder_name
                ),
                deep_link_path: "/admin/affiliate-payouts".to_string(),
                coalesce_key: None,
                collapse_key: Some(format!("wd_req_{}", wd.id)),
                priority_high: true,
                ttl_secs: NotificationCategory::Billing.default_ttl_secs(),
                created_at: now,
                actor: None,
                actors_preview: Vec::new(),
                aggregated_count: 1,
                entity_id: Some(wd.id.to_string()),
                conversation_id: None,
                action_buttons: Vec::new(),
                is_critical_transactional: true,
                attempt: 0,
                first_enqueued_at: now,
            });
        }
    }
}

pub fn push_user_payout_notif(
    state: &AppState,
    target_user_id: Uuid,
    title: &str,
    body: &str,
    wd_id: Uuid,
) {
    let now = Utc::now();
    state.notifications.publish(OmnichannelNotifEvent {
        event_id: Uuid::now_v7(),
        target_user_id,
        category: NotificationCategory::Billing,
        event_type: "AFFILIATE_WITHDRAWAL_UPDATED".to_string(),
        title: title.to_string(),
        body: body.to_string(),
        deep_link_path: "/affiliate".to_string(),
        coalesce_key: None,
        collapse_key: Some(format!("wd_upd_{wd_id}")),
        priority_high: true,
        ttl_secs: NotificationCategory::Billing.default_ttl_secs(),
        created_at: now,
        actor: None,
        actors_preview: Vec::new(),
        aggregated_count: 1,
        entity_id: Some(wd_id.to_string()),
        conversation_id: None,
        action_buttons: Vec::new(),
        is_critical_transactional: true,
        attempt: 0,
        first_enqueued_at: now,
    });
}
