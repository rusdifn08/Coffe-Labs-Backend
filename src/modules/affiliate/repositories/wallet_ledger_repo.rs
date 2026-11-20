use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::affiliate::{
        code_generator::generate_default_affiliate_code,
        models::{AffiliateConfigRow, AffiliateWalletRow, ReferrerLookupRow},
    },
};

pub use crate::modules::affiliate::repositories::ledger_history_repo::{
    has_prior_referral_purchase, has_shared_device_fingerprint, list_recent_conversions,
    list_recent_ledger,
};

const WALLET_COLS: &str = r#"
    user_id, affiliate_code, is_custom_code_claimed, is_affiliate_active,
    available_balance_idr, locked_withdrawal_idr, total_earned_idr, total_withdrawn_idr,
    total_successful_referrals, total_code_uses_count,
    saved_payout_channel::text AS saved_payout_channel,
    saved_account_number, saved_account_holder_name, created_at, updated_at
"#;

pub async fn fetch_program_config(pool: &PgPool) -> AppResult<AffiliateConfigRow> {
    Ok(sqlx::query_as::<_, AffiliateConfigRow>(
        r#"SELECT is_program_active, buyer_discount_bps, affiliate_commission_bps,
                  commission_basis, round_commission_to_nearest_idr,
                  min_withdrawal_idr, max_withdrawal_per_day_idr,
                  withdrawal_admin_fee_idr, first_purchase_only,
                  COALESCE(allow_free_tier_affiliates, FALSE) AS allow_free_tier_affiliates
           FROM affiliate_program_config WHERE config_id = 1"#,
    )
    .fetch_one(pool)
    .await?)
}

pub async fn fetch_user_tier_and_name(
    pool: &PgPool,
    user_id: Uuid,
) -> AppResult<Option<(String, String, String)>> {
    Ok(sqlx::query_as("SELECT tier, role, username FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?)
}

pub async fn find_wallet_by_user(pool: &PgPool, user_id: Uuid) -> AppResult<Option<AffiliateWalletRow>> {
    let sql = format!("SELECT {WALLET_COLS} FROM affiliate_wallets WHERE user_id = $1");
    Ok(sqlx::query_as::<_, AffiliateWalletRow>(&sql).bind(user_id).fetch_optional(pool).await?)
}

/// Pastikan setiap akun Pro / Business / Admin memiliki dompet & kode referral unik otomatis.
pub async fn ensure_wallet_for_user(
    pool: &PgPool,
    user_id: Uuid,
    username: &str,
) -> AppResult<AffiliateWalletRow> {
    if let Some(existing) = find_wallet_by_user(pool, user_id).await? {
        return Ok(existing);
    }
    for _ in 0..8 {
        let code = generate_default_affiliate_code(username);
        let sql = format!(
            "INSERT INTO affiliate_wallets (user_id, affiliate_code)
             VALUES ($1, $2) ON CONFLICT (user_id) DO NOTHING RETURNING {WALLET_COLS}"
        );
        if let Ok(Some(created)) = sqlx::query_as::<_, AffiliateWalletRow>(&sql)
            .bind(user_id)
            .bind(&code)
            .fetch_optional(pool)
            .await
        {
            return Ok(created);
        }
        if let Some(existing) = find_wallet_by_user(pool, user_id).await? {
            return Ok(existing);
        }
    }
    let fallback_code = format!("CL-{}", &user_id.simple().to_string()[..8].to_ascii_uppercase());
    let sql = format!(
        "INSERT INTO affiliate_wallets (user_id, affiliate_code) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET updated_at = NOW() RETURNING {WALLET_COLS}"
    );
    Ok(sqlx::query_as::<_, AffiliateWalletRow>(&sql)
        .bind(user_id)
        .bind(&fallback_code)
        .fetch_one(pool)
        .await?)
}

pub async fn lookup_referrer_by_code(
    pool: &PgPool,
    normalized_code: &str,
) -> AppResult<Option<ReferrerLookupRow>> {
    Ok(sqlx::query_as::<_, ReferrerLookupRow>(
        r#"SELECT w.user_id, w.affiliate_code, w.is_affiliate_active,
                  u.full_name, u.username, u.tier, u.role
           FROM affiliate_wallets w INNER JOIN users u ON u.id = w.user_id
           WHERE UPPER(w.affiliate_code) = UPPER($1) LIMIT 1"#,
    )
    .bind(normalized_code)
    .fetch_optional(pool)
    .await?)
}

pub async fn update_custom_affiliate_code(
    pool: &PgPool,
    user_id: Uuid,
    custom_code: &str,
) -> AppResult<Option<AffiliateWalletRow>> {
    let sql = format!(
        "UPDATE affiliate_wallets
         SET affiliate_code = $1, is_custom_code_claimed = TRUE, updated_at = NOW()
         WHERE user_id = $2 AND is_custom_code_claimed = FALSE RETURNING {WALLET_COLS}"
    );
    Ok(sqlx::query_as::<_, AffiliateWalletRow>(&sql)
        .bind(custom_code)
        .bind(user_id)
        .fetch_optional(pool)
        .await?)
}

pub async fn lock_wallet_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> AppResult<Option<AffiliateWalletRow>> {
    let sql = format!("SELECT {WALLET_COLS} FROM affiliate_wallets WHERE user_id = $1 FOR UPDATE");
    Ok(sqlx::query_as::<_, AffiliateWalletRow>(&sql)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?)
}

pub async fn insert_ledger_entry_idempotent_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    idempotency_key: &str,
    entry_kind: &str,
    available_delta: i64,
    locked_delta: i64,
    available_after: i64,
    locked_after: i64,
    ref_invoice_id: Option<Uuid>,
    ref_withdrawal_id: Option<Uuid>,
    actor_admin_id: Option<Uuid>,
    description: &str,
) -> AppResult<bool> {
    let res = sqlx::query(
        r#"INSERT INTO affiliate_ledger_entries (
            id, user_id, idempotency_key, entry_kind,
            available_delta_idr, locked_delta_idr,
            available_balance_after_idr, locked_balance_after_idr,
            reference_invoice_id, reference_withdrawal_id, actor_admin_id, description
           ) VALUES (
            $1, $2, $3, $4::affiliate_ledger_entry_kind,
            $5, $6, $7, $8, $9, $10, $11, $12
           ) ON CONFLICT (idempotency_key) DO NOTHING"#,
    )
    .bind(Uuid::now_v7()).bind(user_id).bind(idempotency_key).bind(entry_kind)
    .bind(available_delta).bind(locked_delta).bind(available_after).bind(locked_after)
    .bind(ref_invoice_id).bind(ref_withdrawal_id).bind(actor_admin_id).bind(description)
    .execute(&mut **tx)
    .await?;
    Ok(res.rows_affected() > 0)
}
