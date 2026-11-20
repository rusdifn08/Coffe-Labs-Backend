use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ValidateReferralCodeDto {
    pub code: String,
    pub plan_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidatedReferralPreviewDto {
    pub valid: bool,
    pub normalized_code: String,
    pub referrer_display_name: String,
    pub buyer_discount_bps: i32,
    pub buyer_discount_idr: i64,
    pub base_price_idr: i64,
    pub final_base_price_idr: i64,
    pub affiliate_commission_bps: i32,
    pub affiliate_commission_idr: i64,
}

#[derive(Debug, Deserialize)]
pub struct CustomizeCodeDto {
    pub custom_code: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWithdrawalDto {
    pub requested_amount_idr: i64,
    pub payout_channel: String,
    pub bank_or_wallet_label: Option<String>,
    pub account_number: String,
    pub account_holder_name: String,
    pub user_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminApproveWithdrawalDto {
    pub transfer_proof_cloudinary_url: String,
    pub transfer_proof_cloudinary_public_id: Option<String>,
    pub admin_transfer_ref_code: Option<String>,
    pub admin_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminRejectWithdrawalDto {
    pub rejection_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminQueueQueryDto {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PayoutProofSignatureReqDto {
    pub withdrawal_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AffiliateProgramTermsDto {
    pub buyer_discount_bps: i32,
    pub affiliate_commission_bps: i32,
    pub min_withdrawal_idr: i64,
    pub withdrawal_admin_fee_idr: i64,
    pub pro_example_commission_idr: i64,
    pub business_example_commission_idr: i64,
}

#[derive(Debug, Serialize)]
pub struct AffiliateWalletDto {
    pub affiliate_code: String,
    pub share_link: String,
    pub is_custom_code_claimed: bool,
    pub is_affiliate_active: bool,
    pub available_balance_idr: i64,
    pub locked_withdrawal_idr: i64,
    pub total_earned_idr: i64,
    pub total_withdrawn_idr: i64,
    pub total_successful_referrals: i32,
    pub saved_payout_channel: Option<String>,
    pub saved_account_number: Option<String>,
    pub saved_account_holder_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AffiliateConversionDto {
    pub id: Uuid,
    pub buyer_username: String,
    pub buyer_full_name: String,
    pub plan_code: String,
    pub commission_earned_idr: i64,
    pub buyer_discount_idr: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AffiliateWithdrawalDto {
    pub id: Uuid,
    pub ticket_number: String,
    pub requested_amount_idr: i64,
    pub admin_fee_idr: i64,
    pub net_transfer_amount_idr: i64,
    pub payout_channel: String,
    pub bank_or_wallet_label: String,
    pub account_number: String,
    pub account_holder_name: String,
    pub status: String,
    pub sla_deadline_at: DateTime<Utc>,
    pub admin_transfer_ref_code: Option<String>,
    pub transfer_proof_cloudinary_url: Option<String>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_or_rejected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AffiliateLedgerRowDto {
    pub id: Uuid,
    pub entry_kind: String,
    pub available_delta_idr: i64,
    pub locked_delta_idr: i64,
    pub available_balance_after_idr: i64,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AffiliateDashboardDto {
    pub is_eligible_pro: bool,
    pub current_tier: String,
    pub wallet: Option<AffiliateWalletDto>,
    pub terms: AffiliateProgramTermsDto,
    pub recent_conversions: Vec<AffiliateConversionDto>,
    pub recent_withdrawals: Vec<AffiliateWithdrawalDto>,
    pub recent_ledger: Vec<AffiliateLedgerRowDto>,
}

#[derive(Debug, Serialize)]
pub struct AdminWithdrawalItemDto {
    pub id: Uuid,
    pub ticket_number: String,
    pub user_id: Uuid,
    pub affiliate_username: String,
    pub affiliate_full_name: String,
    pub affiliate_code: String,
    pub requested_amount_idr: i64,
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
