use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::core::AppState;
use crate::modules::notifications::events::{NotificationCategory, OmnichannelNotifEvent};
use crate::modules::qris_billing::dto::QrisWsFrame;
use crate::modules::qris_billing::models::{QrisInvoiceRow, UpgradedUserRow};
use crate::modules::qris_billing::services::amount_allocator_service::set_paid_amount_cooldown;
use crate::modules::qris_billing::services::qris_ws_hub::global_qris_ws_hub;

/// Menjalankan seluruh efek pasca-pelunasan ACID: cooldown Redis, invalidasi cache profil,
/// cetak JWT baru dengan klaim tier terbaru, siaran WebSocket, RabbitMQ, dan Push FCM.
pub async fn dispatch_upgrade_side_effects(
    state: &AppState,
    invoice: &QrisInvoiceRow,
    upgraded: &UpgradedUserRow,
    channel: &str,
) -> AppResult<String> {
    let redis_pool = state.redis.active_pool();
    set_paid_amount_cooldown(redis_pool.as_ref(), invoice.total_payable_idr).await;
    state.social_profile_cqrs.invalidate_profile(upgraded.id).await;

    let fresh_access_token = state.jwt.generate_user_token(
        upgraded.id,
        &upgraded.username,
        &upgraded.role,
        &upgraded.tier,
        &upgraded.status,
    )?;

    let expires_at = upgraded
        .subscription_expires_at
        .unwrap_or_else(|| Utc::now() + chrono::Duration::days(i64::from(invoice.duration_days)));

    global_qris_ws_hub()
        .publish(
            redis_pool.as_ref(),
            invoice.id,
            QrisWsFrame::PaymentConfirmed {
                invoice_id: invoice.id,
                invoice_number: invoice.invoice_number.clone(),
                new_tier: upgraded.tier.clone(),
                plan_code: invoice.plan_code.clone(),
                subscription_expires_at: expires_at,
                fresh_access_token: fresh_access_token.clone(),
            },
        )
        .await;

    let tier_label = if upgraded.tier.eq_ignore_ascii_case("business") {
        "Coffee Labs Business"
    } else {
        "Coffee Labs Pro"
    };

    state.rabbitmq.publish_json(
        "",
        "billing.subscription.upgraded",
        &json!({
            "event": "SubscriptionUpgradedEvent",
            "invoice_id": invoice.id,
            "invoice_number": invoice.invoice_number,
            "user_id": upgraded.id,
            "new_tier": upgraded.tier,
            "plan_code": invoice.plan_code,
            "amount_idr": invoice.total_payable_idr,
            "channel": channel,
            "subscription_expires_at": expires_at,
        }),
    );

    let now = Utc::now();
    state.notifications.publish(OmnichannelNotifEvent {
        event_id: Uuid::now_v7(),
        target_user_id: upgraded.id,
        category: NotificationCategory::Billing,
        event_type: "QRIS_SUBSCRIPTION_ACTIVATED".to_string(),
        title: format!("{tier_label} Aktif!"),
        body: format!(
            "Pembayaran QRIS Rp{} ({}) berhasil diverifikasi. Seluruh fitur {tier_label} kini terbuka.",
            invoice.total_payable_idr, invoice.invoice_number
        ),
        deep_link_path: "/profile".to_string(),
        coalesce_key: None,
        collapse_key: Some(format!("qris_paid_{}", invoice.id)),
        priority_high: true,
        ttl_secs: NotificationCategory::Billing.default_ttl_secs(),
        created_at: now,
        actor: None,
        actors_preview: Vec::new(),
        aggregated_count: 1,
        entity_id: Some(invoice.id.to_string()),
        conversation_id: None,
        action_buttons: Vec::new(),
        is_critical_transactional: true,
        attempt: 0,
        first_enqueued_at: now,
    });

    Ok(fresh_access_token)
}
