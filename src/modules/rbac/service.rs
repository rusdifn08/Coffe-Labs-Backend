use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::{AppError, AppResult},
    modules::rbac::{
        dto::{QuotaStatusResponse, SubscriptionPlanDto, UpgradeTierRequest},
        error::RbacError,
        guard::permissions_for_tier,
        models::{AccountTier, UserRbacState},
        repository,
    },
};

pub const MAX_TRIAL_BREWS: i32 = 10;
pub const MAX_TRIAL_RECIPES: i32 = 10;

pub fn get_all_plans() -> Vec<SubscriptionPlanDto> {
    vec![
        SubscriptionPlanDto {
            id: "plan_trial".to_string(),
            tier: AccountTier::Trial,
            title: "Coffee Labs Trial".to_string(),
            subtitle: "Akses Eksplorasi Awal Barista".to_string(),
            price_label: "Gratis".to_string(),
            price_amount: 0,
            billing_period: "Selamanya (Kapasitas Terbatas)".to_string(),
            badge_label: "TRIAL BARISTA".to_string(),
            is_recommended: false,
            features: vec![
                "Batas 10 kali seduh aktif".to_string(),
                "Akses 10 resep dasar pilihan".to_string(),
                "Timer seduh standar".to_string(),
            ],
        },
        SubscriptionPlanDto {
            id: "plan_coffe_labs_pro".to_string(),
            tier: AccountTier::Pro,
            title: "Coffee Labs Pro".to_string(),
            subtitle: "Solusi Lengkap Home Brewer & Specialist".to_string(),
            price_label: "Rp 99.000".to_string(),
            price_amount: 99000,
            billing_period: "Sekali Bayar • Akses Penuh Selamanya".to_string(),
            badge_label: "COFFEE LABS PRO".to_string(),
            is_recommended: true,
            features: vec![
                "Bebas Iklan 100% Selamanya (Zero Ads)".to_string(),
                "Buka Seluruh 409+ Resep Juara Dunia (WBC / WBrC)".to_string(),
                "Seduh Tanpa Batas (Unlimited Brews)".to_string(),
                "Lab Ekstraksi: Dial-In Assistant V60 & Espresso".to_string(),
                "Grinder Matrix 8 Grinder Dunia (µm Slider)".to_string(),
            ],
        },
        SubscriptionPlanDto {
            id: "plan_coffe_labs_business".to_string(),
            tier: AccountTier::Business,
            title: "Coffee Labs Business".to_string(),
            subtitle: "Ekosistem Profesional untuk Kafe & Micro-Roastery".to_string(),
            price_label: "Rp 249.000".to_string(),
            price_amount: 249000,
            billing_period: "Sekali Bayar • Lisensi Komersial Bisnis".to_string(),
            badge_label: "BUSINESS CAFE & ROASTER".to_string(),
            is_recommended: false,
            features: vec![
                "Seluruh Fitur Coffee Labs Pro Termasuk".to_string(),
                "Multi-Barista Sync (Hingga 5 Akun Barista Toko)".to_string(),
                "Cetak Struk Seduh Bluetooth Thermal Printer Tanpa Batas".to_string(),
                "Export Jurnal Evaluasi Sensori PDF & CSV".to_string(),
                "Beans Radar & Marketplace Priority Listing".to_string(),
            ],
        },
    ]
}

fn map_state_to_quota(state: UserRbacState) -> QuotaStatusResponse {
    let tier = if state.role == "admin" {
        AccountTier::Business
    } else {
        AccountTier::from_str_lossy(&state.tier)
    };
    let is_pro_or_business = matches!(tier, AccountTier::Pro | AccountTier::Business);
    let is_business = tier == AccountTier::Business;

    let now = Utc::now();
    let is_brew_locked = if is_pro_or_business {
        false
    } else if let Some(until) = state.trial_locked_until {
        until > now && state.trial_brews_count >= MAX_TRIAL_BREWS
    } else {
        state.trial_brews_count >= MAX_TRIAL_BREWS
    };

    let remaining = if is_pro_or_business {
        999999
    } else {
        (MAX_TRIAL_BREWS - state.trial_brews_count).max(0)
    };

    QuotaStatusResponse {
        tier,
        plan_id: state
            .subscription_plan_id
            .unwrap_or_else(|| tier.plan_id().to_string()),
        is_pro_or_business,
        is_business,
        max_trial_brews: MAX_TRIAL_BREWS,
        max_trial_recipes: MAX_TRIAL_RECIPES,
        current_trial_brews: state.trial_brews_count,
        remaining_trial_brews: remaining,
        is_brew_locked,
        trial_locked_until: state.trial_locked_until,
        permissions: permissions_for_tier(tier),
        activated_at: state.subscription_activated_at,
        payment_method: state.subscription_payment_method,
    }
}

pub async fn get_subscription_status(pool: &PgPool, user_id: Uuid) -> AppResult<QuotaStatusResponse> {
    let mut state = repository::get_user_rbac(pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User tidak ditemukan.".to_string()))?;

    if let Some(until) = state.trial_locked_until {
        if Utc::now() >= until {
            state = repository::update_trial_counters(pool, user_id, 0, None).await?;
        }
    }

    Ok(map_state_to_quota(state))
}

pub async fn upgrade_user_tier(
    pool: &PgPool,
    user_id: Uuid,
    req: UpgradeTierRequest,
) -> AppResult<QuotaStatusResponse> {
    let target_tier = match req.plan_id.trim().to_lowercase().as_str() {
        "pro" | "plan_coffe_labs_pro" => AccountTier::Pro,
        "business" | "plan_coffe_labs_business" => AccountTier::Business,
        "trial" | "plan_trial" => AccountTier::Trial,
        other => return Err(RbacError::InvalidPlan(other.to_string()).into()),
    };

    let payment = req.payment_method.as_deref().unwrap_or("QRIS Instant");
    let state = repository::update_user_tier(
        pool,
        user_id,
        target_tier.as_str(),
        target_tier.plan_id(),
        payment,
    )
    .await?;

    Ok(map_state_to_quota(state))
}

pub async fn record_brew_and_enforce_quota(
    pool: &PgPool,
    user_id: Uuid,
) -> AppResult<QuotaStatusResponse> {
    let mut state = repository::get_user_rbac(pool, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User tidak ditemukan.".to_string()))?;

    let tier = if state.role == "admin" {
        AccountTier::Business
    } else {
        AccountTier::from_str_lossy(&state.tier)
    };

    if matches!(tier, AccountTier::Pro | AccountTier::Business) {
        return Ok(map_state_to_quota(state));
    }

    if let Some(until) = state.trial_locked_until {
        if Utc::now() >= until {
            state = repository::update_trial_counters(pool, user_id, 0, None).await?;
        } else if state.trial_brews_count >= MAX_TRIAL_BREWS {
            return Err(RbacError::TrialBrewQuotaLocked.into());
        }
    }

    let next_count = state.trial_brews_count + 1;
    let lock_until = if next_count >= MAX_TRIAL_BREWS {
        Some(Utc::now() + Duration::days(7))
    } else {
        None
    };

    let updated = repository::update_trial_counters(pool, user_id, next_count, lock_until).await?;
    Ok(map_state_to_quota(updated))
}
