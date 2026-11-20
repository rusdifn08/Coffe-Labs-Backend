use crate::modules::affiliate::{
    dto::{
        AdminWithdrawalItemDto, AffiliateConversionDto, AffiliateLedgerRowDto,
        AffiliateProgramTermsDto, AffiliateWalletDto, AffiliateWithdrawalDto,
    },
    math_and_sla::compute_checkout_affiliate_pricing,
    models::{
        AdminWithdrawalQueueRow, AffiliateConfigRow, AffiliateConversionRow, AffiliateLedgerRow,
        AffiliateWalletRow, AffiliateWithdrawalRow,
    },
};

impl From<AffiliateWalletRow> for AffiliateWalletDto {
    fn from(w: AffiliateWalletRow) -> Self {
        let share_link = format!("https://coffeelabs.id/join?ref={}", w.affiliate_code);
        Self {
            affiliate_code: w.affiliate_code,
            share_link,
            is_custom_code_claimed: w.is_custom_code_claimed,
            is_affiliate_active: w.is_affiliate_active,
            available_balance_idr: w.available_balance_idr,
            locked_withdrawal_idr: w.locked_withdrawal_idr,
            total_earned_idr: w.total_earned_idr,
            total_withdrawn_idr: w.total_withdrawn_idr,
            total_successful_referrals: w.total_successful_referrals,
            saved_payout_channel: w.saved_payout_channel,
            saved_account_number: w.saved_account_number,
            saved_account_holder_name: w.saved_account_holder_name,
        }
    }
}

impl From<AffiliateConversionRow> for AffiliateConversionDto {
    fn from(r: AffiliateConversionRow) -> Self {
        Self {
            id: r.id,
            buyer_username: r.buyer_username,
            buyer_full_name: r.buyer_full_name,
            plan_code: r.plan_code,
            commission_earned_idr: r.commission_earned_idr,
            buyer_discount_idr: r.buyer_discount_idr,
            created_at: r.created_at,
        }
    }
}

impl From<AffiliateWithdrawalRow> for AffiliateWithdrawalDto {
    fn from(r: AffiliateWithdrawalRow) -> Self {
        Self {
            id: r.id,
            ticket_number: r.ticket_number,
            requested_amount_idr: r.requested_amount_idr,
            admin_fee_idr: r.admin_fee_idr,
            net_transfer_amount_idr: r.net_transfer_amount_idr,
            payout_channel: r.payout_channel,
            bank_or_wallet_label: r.bank_or_wallet_label,
            account_number: r.account_number,
            account_holder_name: r.account_holder_name,
            status: r.status,
            sla_deadline_at: r.sla_deadline_at,
            admin_transfer_ref_code: r.admin_transfer_ref_code,
            transfer_proof_cloudinary_url: r.transfer_proof_cloudinary_url,
            rejection_reason: r.rejection_reason,
            created_at: r.created_at,
            completed_or_rejected_at: r.completed_or_rejected_at,
        }
    }
}

impl From<AffiliateLedgerRow> for AffiliateLedgerRowDto {
    fn from(r: AffiliateLedgerRow) -> Self {
        Self {
            id: r.id,
            entry_kind: r.entry_kind,
            available_delta_idr: r.available_delta_idr,
            locked_delta_idr: r.locked_delta_idr,
            available_balance_after_idr: r.available_balance_after_idr,
            description: r.description,
            created_at: r.created_at,
        }
    }
}

impl From<AdminWithdrawalQueueRow> for AdminWithdrawalItemDto {
    fn from(r: AdminWithdrawalQueueRow) -> Self {
        Self {
            id: r.id,
            ticket_number: r.ticket_number,
            user_id: r.user_id,
            affiliate_username: r.affiliate_username,
            affiliate_full_name: r.affiliate_full_name,
            affiliate_code: r.affiliate_code,
            requested_amount_idr: r.requested_amount_idr,
            net_transfer_amount_idr: r.net_transfer_amount_idr,
            payout_channel: r.payout_channel,
            bank_or_wallet_label: r.bank_or_wallet_label,
            account_number: r.account_number,
            account_holder_name: r.account_holder_name,
            user_notes: r.user_notes,
            status: r.status,
            sla_deadline_at: r.sla_deadline_at,
            admin_transfer_ref_code: r.admin_transfer_ref_code,
            transfer_proof_cloudinary_url: r.transfer_proof_cloudinary_url,
            rejection_reason: r.rejection_reason,
            created_at: r.created_at,
            completed_or_rejected_at: r.completed_or_rejected_at,
        }
    }
}

pub fn build_terms_dto(cfg: &AffiliateConfigRow) -> AffiliateProgramTermsDto {
    let pro = compute_checkout_affiliate_pricing(99_000, cfg);
    let biz = compute_checkout_affiliate_pricing(249_000, cfg);
    AffiliateProgramTermsDto {
        buyer_discount_bps: cfg.buyer_discount_bps,
        affiliate_commission_bps: cfg.affiliate_commission_bps,
        min_withdrawal_idr: cfg.min_withdrawal_idr,
        withdrawal_admin_fee_idr: cfg.withdrawal_admin_fee_idr,
        pro_example_commission_idr: pro.affiliate_commission_idr,
        business_example_commission_idr: biz.affiliate_commission_idr,
    }
}
