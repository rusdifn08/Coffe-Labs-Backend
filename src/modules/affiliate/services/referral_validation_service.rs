use fred::interfaces::KeysInterface;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::{
        affiliate::{
            code_generator::is_pro_eligible_tier,
            dto::ValidatedReferralPreviewDto,
            error::AffiliateError,
            math_and_sla::{compute_checkout_affiliate_pricing, AffiliatePricingBreakdown},
            models::ReferrerLookupRow,
            repositories::wallet_ledger_repo,
        },
        qris_billing::{
            handlers::checkout_handler::normalize_lifetime_plan_code,
            repositories::invoice_repo,
        },
    },
};

pub struct ValidatedReferralContext {
    pub referrer: ReferrerLookupRow,
    pub pricing: AffiliatePricingBreakdown,
}

async fn check_validate_rate_limit(state: &AppState, buyer_user_id: Uuid) -> Result<(), AffiliateError> {
    let Some(pool) = state.redis.active_pool() else {
        return Ok(());
    };
    let key = format!("aff:val_rl:{buyer_user_id}");
    if let Ok(Ok(count)) = tokio::time::timeout(
        std::time::Duration::from_millis(180),
        pool.incr::<i64, _>(&key),
    )
    .await
    {
        if count == 1 {
            let _ = tokio::time::timeout(
                std::time::Duration::from_millis(180),
                pool.expire::<(), _>(&key, 60),
            )
            .await;
        }
        if count > 10 {
            return Err(AffiliateError::RateLimited);
        }
    }
    Ok(())
}

/// Anti-Fraud 4-Lapis + Kalkulasi Diskon Pembeli & Snapshot Komisi 20% Afiliator Pro.
pub async fn validate_referral_for_checkout(
    state: &AppState,
    buyer_user_id: Uuid,
    raw_code: &str,
    plan_code_opt: Option<&str>,
) -> AppResult<ValidatedReferralContext> {
    let normalized = raw_code.trim().to_ascii_uppercase();
    if normalized.len() < 4 || normalized.len() > 24 {
        return Err(AffiliateError::InvalidCodeFormat(
            "Format kode referral tidak valid.".into(),
        )
        .into());
    }

    // Layer 4: Rate Limit Guessing (max 10x / menit per user)
    check_validate_rate_limit(state, buyer_user_id).await?;

    let cfg = wallet_ledger_repo::fetch_program_config(&state.pool).await?;
    if !cfg.is_program_active {
        return Err(AffiliateError::ProgramInactive.into());
    }

    let referrer = wallet_ledger_repo::lookup_referrer_by_code(&state.pool, &normalized)
        .await?
        .ok_or(AffiliateError::CodeNotFound)?;

    // Hanya pemilik akun Pro / Business / Admin aktif yang berhak menjadi afiliator!
    if !referrer.is_affiliate_active || !is_pro_eligible_tier(&referrer.tier, &referrer.role) {
        return Err(AffiliateError::CodeNotFound.into());
    }

    // Layer 1: Self-Referral Blocker
    if referrer.user_id == buyer_user_id {
        return Err(AffiliateError::SelfReferralForbidden.into());
    }

    // Layer 2: Same-Device Fingerprint Collision Guard (90 hari)
    if wallet_ledger_repo::has_shared_device_fingerprint(&state.pool, buyer_user_id, referrer.user_id)
        .await?
    {
        return Err(AffiliateError::SameDeviceFraudBlocked.into());
    }

    // Layer 3: First Purchase Only / Anti Mutual-Ring Guard
    if cfg.first_purchase_only
        && wallet_ledger_repo::has_prior_referral_purchase(&state.pool, buyer_user_id).await?
    {
        return Err(AffiliateError::CircularReferralBlocked.into());
    }

    let requested_plan = plan_code_opt.unwrap_or("PRO_LIFETIME").trim().to_ascii_uppercase();
    let plan_row = match invoice_repo::find_plan_by_code(&state.pool, &requested_plan).await? {
        Some(exact) => Some(exact),
        None => {
            let fallback = normalize_lifetime_plan_code(&requested_plan);
            invoice_repo::find_plan_by_code(&state.pool, &fallback).await?
        }
    };
    let base_price_idr = match plan_row {
        Some(p) => p.base_price_idr,
        None => {
            if requested_plan.contains("BUSINESS") {
                249_000
            } else {
                99_000
            }
        }
    };

    let pricing = compute_checkout_affiliate_pricing(base_price_idr, &cfg);
    Ok(ValidatedReferralContext { referrer, pricing })
}

pub fn to_preview_dto(ctx: &ValidatedReferralContext) -> ValidatedReferralPreviewDto {
    let display = if ctx.referrer.full_name.trim().is_empty() {
        format!("@{}", ctx.referrer.username)
    } else {
        ctx.referrer.full_name.clone()
    };
    ValidatedReferralPreviewDto {
        valid: true,
        normalized_code: ctx.referrer.affiliate_code.clone(),
        referrer_display_name: display,
        buyer_discount_bps: ctx.pricing.buyer_discount_bps,
        buyer_discount_idr: ctx.pricing.buyer_discount_idr,
        base_price_idr: ctx.pricing.base_price_idr,
        final_base_price_idr: ctx.pricing.discounted_price_idr,
        affiliate_commission_bps: ctx.pricing.affiliate_commission_bps,
        affiliate_commission_idr: ctx.pricing.affiliate_commission_idr,
    }
}
