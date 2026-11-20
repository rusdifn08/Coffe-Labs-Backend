use axum::{extract::State, Json};

use crate::{
    core::{
        auth_extractor::AuthenticatedUser,
        error::{ApiResponse, AppError, AppResult},
        AppState,
    },
    modules::affiliate::{
        code_generator::{is_pro_eligible_tier, validate_and_normalize_custom_code},
        dto::{
            AffiliateConversionDto, AffiliateDashboardDto, AffiliateLedgerRowDto,
            AffiliateWalletDto, AffiliateWithdrawalDto, CustomizeCodeDto,
            ValidateReferralCodeDto, ValidatedReferralPreviewDto,
        },
        dto_mappers::build_terms_dto,
        error::AffiliateError,
        repositories::{wallet_ledger_repo, withdrawal_repo},
        services::referral_validation_service,
    },
};

/// GET /api/v1/affiliate/me — Dashboard Dompet Afiliasi (otomatis aktivasi kode untuk akun Pro/Business/Admin).
pub async fn get_my_affiliate_dashboard(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<AffiliateDashboardDto>>> {
    let cfg = wallet_ledger_repo::fetch_program_config(&state.pool).await?;
    let terms = build_terms_dto(&cfg);
    let (tier, role, username) = wallet_ledger_repo::fetch_user_tier_and_name(&state.pool, auth.user_id)
        .await?
        .unwrap_or_else(|| (auth.claims.tier.clone(), auth.claims.role.clone(), auth.claims.username.clone()));

    let eligible = is_pro_eligible_tier(&tier, &role);
    if !eligible {
        return Ok(ApiResponse::ok(
            "Program Afiliasi 20% tersedia eksklusif untuk akun Coffee Labs Pro & Business.",
            AffiliateDashboardDto {
                is_eligible_pro: false,
                current_tier: tier,
                wallet: None,
                terms,
                recent_conversions: Vec::new(),
                recent_withdrawals: Vec::new(),
                recent_ledger: Vec::new(),
            },
        ));
    }

    let wallet_row = wallet_ledger_repo::ensure_wallet_for_user(&state.pool, auth.user_id, &username).await?;
    let conversions = wallet_ledger_repo::list_recent_conversions(&state.pool, auth.user_id, 50).await?;
    let withdrawals = withdrawal_repo::list_user_withdrawals(&state.pool, auth.user_id, 30).await?;
    let ledger = wallet_ledger_repo::list_recent_ledger(&state.pool, auth.user_id, 50).await?;

    Ok(ApiResponse::ok(
        "Dashboard Program Afiliasi Coffee Labs berhasil dimuat.",
        AffiliateDashboardDto {
            is_eligible_pro: true,
            current_tier: tier,
            wallet: Some(AffiliateWalletDto::from(wallet_row)),
            terms,
            recent_conversions: conversions.into_iter().map(AffiliateConversionDto::from).collect(),
            recent_withdrawals: withdrawals.into_iter().map(AffiliateWithdrawalDto::from).collect(),
            recent_ledger: ledger.into_iter().map(AffiliateLedgerRowDto::from).collect(),
        },
    ))
}

/// PUT /api/v1/affiliate/code — Kustomisasi Vanity Code 1x Gratis (khusus akun Pro/Business/Admin).
pub async fn customize_my_affiliate_code(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<CustomizeCodeDto>,
) -> AppResult<Json<ApiResponse<AffiliateWalletDto>>> {
    let (tier, role, username) = wallet_ledger_repo::fetch_user_tier_and_name(&state.pool, auth.user_id)
        .await?
        .ok_or(AffiliateError::ProAccountRequired)?;
    if !is_pro_eligible_tier(&tier, &role) {
        return Err(AffiliateError::ProAccountRequired.into());
    }

    let normalized = validate_and_normalize_custom_code(&req.custom_code)?;
    let _ = wallet_ledger_repo::ensure_wallet_for_user(&state.pool, auth.user_id, &username).await?;

    if let Some(existing) = wallet_ledger_repo::lookup_referrer_by_code(&state.pool, &normalized).await? {
        if existing.user_id != auth.user_id {
            return Err(AffiliateError::CodeAlreadyTaken.into());
        }
    }

    let updated = wallet_ledger_repo::update_custom_affiliate_code(&state.pool, auth.user_id, &normalized)
        .await
        .map_err(|e| match e {
            AppError::Database(_) => AppError::from(AffiliateError::CodeAlreadyTaken),
            other => other,
        })?
        .ok_or(AffiliateError::CustomCodeAlreadyClaimed)?;

    Ok(ApiResponse::ok(
        "Kode referral kustom Anda berhasil diperbarui!",
        AffiliateWalletDto::from(updated),
    ))
}

/// POST /api/v1/affiliate/validate-code — Validasi kode & pratinjau diskon checkout QRIS (terbuka untuk semua pembeli).
pub async fn validate_referral_code(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<ValidateReferralCodeDto>,
) -> AppResult<Json<ApiResponse<ValidatedReferralPreviewDto>>> {
    let ctx = referral_validation_service::validate_referral_for_checkout(
        &state,
        auth.user_id,
        &req.code,
        req.plan_code.as_deref(),
    )
    .await?;

    Ok(ApiResponse::ok(
        "Kode referral valid! Diskon pembeli siap diterapkan saat checkout.",
        referral_validation_service::to_preview_dto(&ctx),
    ))
}
