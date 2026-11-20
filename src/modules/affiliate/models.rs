#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct AffiliateConfigRow {
    pub is_program_active: bool,
    pub buyer_discount_bps: i32,
    pub affiliate_commission_bps: i32,
    pub commission_basis: String,
    pub round_commission_to_nearest_idr: i32,
    pub min_withdrawal_idr: i64,
    pub max_withdrawal_per_day_idr: i64,
    pub withdrawal_admin_fee_idr: i64,
    pub first_purchase_only: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct AffiliateWalletRow {
    pub user_id: Uuid,
    pub affiliate_code: String,
    pub is_custom_code_claimed: bool,
    pub is_affiliate_active: bool,
    pub available_balance_idr: i64,
    pub locked_withdrawal_idr: i64,
    pub total_earned_idr: i64,
    pub total_withdrawn_idr: i64,
    pub total_successful_referrals: i32,
    pub total_code_uses_count: i32,
    pub saved_payout_channel: Option<String>,
    pub saved_account_number: Option<String>,
    pub saved_account_holder_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ReferrerLookupRow {
    pub user_id: Uuid,
    pub affiliate_code: String,
    pub is_affiliate_active: bool,
    pub full_name: String,
    pub username: String,
    pub tier: String,
    pub role: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct AffiliateConversionRow {
    pub id: Uuid,
    pub buyer_username: String,
    pub buyer_full_name: String,
    pub affiliate_code_used: String,
    pub plan_code: String,
    pub plan_base_price_idr: i64,
    pub buyer_discount_idr: i64,
    pub commission_bps: i32,
    pub commission_earned_idr: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AffiliateWithdrawalRow {
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
    pub status: String,
    pub sla_deadline_at: DateTime<Utc>,
    pub reviewed_by_admin_id: Option<Uuid>,
    pub admin_transfer_ref_code: Option<String>,
    pub transfer_proof_cloudinary_url: Option<String>,
    pub admin_notes: Option<String>,
    pub rejection_reason: Option<String>,
    pub processed_at: Option<DateTime<Utc>>,
    pub completed_or_rejected_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AdminWithdrawalQueueRow {
    pub id: Uuid,
    pub ticket_number: String,
    pub user_id: Uuid,
    pub affiliate_username: String,
    pub affiliate_full_name: String,
    pub affiliate_code: String,
    pub requested_amount_idr: i64,
    pub admin_fee_idr: i64,
    pub net_transfer_amount_idr: i64,
    pub payout_channel: String,
    pub bank_or_wallet_label: String,
    pub account_number: String,
    pub account_holder_name: String,
    pub user_notes: Option<String>,
    pub status: String,
    pub sla_deadline_at: DateTime<Utc>,
    pub admin_transfer_ref_code: Option<String>,
    pub transfer_proof_cloudinary_url: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_or_rejected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AffiliateLedgerRow {
    pub id: Uuid,
    pub entry_kind: String,
    pub available_delta_idr: i64,
    pub locked_delta_idr: i64,
    pub available_balance_after_idr: i64,
    pub locked_balance_after_idr: i64,
    pub description: String,
    pub created_at: DateTime<Utc>,
}
