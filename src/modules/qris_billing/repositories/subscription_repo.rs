use chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::modules::qris_billing::models::UpgradedUserRow;

pub struct NewMutationParams {
    pub id: Uuid,
    pub source_channel: String,
    pub source_fingerprint: String,
    pub amount_idr: i64,
    pub payer_name: Option<String>,
    pub bank_rrn_ref: Option<String>,
    pub raw_notification_text: String,
    pub received_at: DateTime<Utc>,
}

/// Menyisipkan mutasi masuk secara 100% idempoten (`ON CONFLICT DO NOTHING`).
/// Mengembalikan `Some(id)` jika mutasi baru, atau `None` jika duplikat dari channel ganda.
pub async fn insert_mutation_idempotent_tx(
    tx: &mut Transaction<'_, Postgres>,
    m: NewMutationParams,
) -> AppResult<Option<Uuid>> {
    let inserted: Option<Uuid> = sqlx::query_scalar(
        r#"INSERT INTO qris_payment_mutations (
               id, source_channel, source_fingerprint, amount_idr,
               payer_name, bank_rrn_ref, raw_notification_text, received_at,
               reconciliation_status
           ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'UNMATCHED')
           ON CONFLICT (source_fingerprint) DO NOTHING
           RETURNING id"#,
    )
    .bind(m.id)
    .bind(m.source_channel)
    .bind(m.source_fingerprint)
    .bind(m.amount_idr)
    .bind(m.payer_name)
    .bind(m.bank_rrn_ref)
    .bind(m.raw_notification_text)
    .bind(m.received_at)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(inserted)
}

pub async fn mark_invoice_and_mutation_paid_tx(
    tx: &mut Transaction<'_, Postgres>,
    invoice_id: Uuid,
    mutation_id: Uuid,
    reconciliation_status: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE qris_subscription_invoices
         SET status = 'PAID'::qris_invoice_status,
             paid_at = NOW(),
             matched_mutation_id = $2,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(invoice_id)
    .bind(mutation_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "UPDATE qris_payment_mutations
         SET matched_invoice_id = $2,
             reconciliation_status = $3
         WHERE id = $1",
    )
    .bind(mutation_id)
    .bind(invoice_id)
    .bind(reconciliation_status)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Meng-upgrade akun `users` secara atomik ke `pro` atau `business`, memperpanjang masa aktif,
/// dan menyinkronkan proyeksi `user_profile_projections` serta `account_monitoring_dossiers`.
pub async fn upgrade_user_subscription_atomic_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    target_tier: &str,
    plan_code: &str,
    duration_days: i32,
) -> AppResult<UpgradedUserRow> {
    let lower_tier = if target_tier.eq_ignore_ascii_case("business") {
        "business"
    } else {
        "pro"
    };
    let title_tier = if lower_tier == "business" {
        "Business"
    } else {
        "Pro"
    };
    let legacy_plan_id = if lower_tier == "business" {
        "plan_coffe_labs_business"
    } else {
        "plan_coffe_labs_pro"
    };

    let upgraded = sqlx::query_as::<_, UpgradedUserRow>(
        r#"UPDATE users
           SET tier = CASE WHEN tier = 'business' AND $2 = 'pro' THEN 'business' ELSE $2 END,
               subscription_plan_id = $3,
               subscription_payment_method = 'QRIS_DINAMIS_LIVIN',
               subscription_activated_at = NOW(),
               subscription_expires_at = GREATEST(COALESCE(subscription_expires_at, NOW()), NOW()) + make_interval(days => $4),
               active_plan_code = $5::subscription_plan_code,
               trial_policy_override = NULL,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, username, full_name, role, status, tier,
                     subscription_plan_id, subscription_expires_at,
                     active_plan_code::text AS active_plan_code"#,
    )
    .bind(user_id)
    .bind(lower_tier)
    .bind(legacy_plan_id)
    .bind(duration_days.max(1))
    .bind(plan_code)
    .fetch_one(&mut **tx)
    .await?;

    let _ = sqlx::query(
        "UPDATE user_profile_projections
         SET account_tier = $2, updated_at = NOW()
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(title_tier)
    .execute(&mut **tx)
    .await;

    let _ = sqlx::query(
        "UPDATE account_monitoring_dossiers
         SET account_tier = $2, updated_at = NOW()
         WHERE user_id = $1",
    )
    .bind(user_id)
    .bind(title_tier)
    .execute(&mut **tx)
    .await;

    Ok(upgraded)
}

pub async fn fetch_user_for_token(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> AppResult<Option<UpgradedUserRow>> {
    let row = sqlx::query_as::<_, UpgradedUserRow>(
        r#"SELECT id, username, full_name, role, status, tier,
                  subscription_plan_id, subscription_expires_at,
                  active_plan_code::text AS active_plan_code
           FROM users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
