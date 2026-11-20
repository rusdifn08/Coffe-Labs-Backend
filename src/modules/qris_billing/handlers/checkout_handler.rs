use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::core::{
    auth_extractor::AuthenticatedUser,
    error::{ApiResponse, AppError, AppResult},
    AppState,
};
use crate::modules::{
    affiliate::services::referral_validation_service,
    qris_billing::{
        config::QrisBillingConfig,
        dto::{CreateQrisCheckoutDto, DynamicQrisInvoiceDto, SubscriptionPlanDto},
        emvco_qris::{build_dynamic_qris, extract_merchant_meta},
        error::QrisBillingError,
        repositories::{
            invoice_repo::{self, NewInvoiceParams},
            subscription_repo,
        },
        services::amount_allocator_service::allocate_unique_suffix,
    },
};

pub fn normalize_lifetime_plan_code(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "PRO" | "PRO_MONTHLY" | "PRO_YEARLY" | "PRO_LIFETIME" | "PLAN_COFFE_LABS_PRO" => {
            "PRO_LIFETIME".to_string()
        }
        "BUSINESS"
        | "BUSINESS_MONTHLY"
        | "BUSINESS_YEARLY"
        | "BUSINESS_LIFETIME"
        | "PLAN_COFFE_LABS_BUSINESS" => "BUSINESS_LIFETIME".to_string(),
        other => other.to_string(),
    }
}

pub async fn list_plans(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<SubscriptionPlanDto>>>> {
    let rows = invoice_repo::list_active_plans(&state.pool).await?;
    let plans = rows.into_iter().map(SubscriptionPlanDto::from).collect();
    Ok(ApiResponse::ok(
        "Daftar paket Lifetime Coffee Labs Pro & Business (Promo Akhir 2026).",
        plans,
    ))
}

pub async fn create_or_resume_checkout(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<CreateQrisCheckoutDto>,
) -> AppResult<Json<ApiResponse<DynamicQrisInvoiceDto>>> {
    let requested_code = req.plan_code.trim().to_ascii_uppercase();
    let plan = match invoice_repo::find_plan_by_code(&state.pool, &requested_code).await? {
        Some(exact) => exact,
        None => {
            let fallback_code = normalize_lifetime_plan_code(&requested_code);
            invoice_repo::find_plan_by_code(&state.pool, &fallback_code)
                .await?
                .ok_or_else(|| QrisBillingError::UnknownPlanCode(fallback_code))?
        }
    };
    let plan_code = plan.plan_code.clone();

    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);

    let clean_ref = req
        .referral_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    // Jika user tidak mengganti kode referral, lanjutkan invoice pending yang masih aktif
    if clean_ref.is_none() {
        if let Some(existing) =
            invoice_repo::find_active_pending_for_user_plan(&state.pool, auth.user_id, &plan_code)
                .await?
        {
            return Ok(ApiResponse::ok(
                "Melanjutkan invoice QRIS aktif.",
                DynamicQrisInvoiceDto::from_row(existing, meta.nmid),
            ));
        }
    }

    let (applied_code, referrer_id, discount_idr, comm_bps, comm_idr, payable_base) =
        if let Some(code_str) = clean_ref {
            let ctx = referral_validation_service::validate_referral_for_checkout(
                &state,
                auth.user_id,
                code_str,
                Some(&plan_code),
            )
            .await?;
            (
                Some(ctx.referrer.affiliate_code),
                Some(ctx.referrer.user_id),
                ctx.pricing.buyer_discount_idr,
                ctx.pricing.affiliate_commission_bps,
                ctx.pricing.affiliate_commission_idr,
                ctx.pricing.discounted_price_idr,
            )
        } else {
            (None, None, 0, 0, 0, plan.base_price_idr)
        };

    let invoice_id = Uuid::now_v7();
    let redis_pool = state.redis.active_pool();
    let (unique_code, total_payable) =
        allocate_unique_suffix(&state.pool, redis_pool.as_ref(), payable_base, invoice_id).await?;

    let dynamic_payload = build_dynamic_qris(&cfg.master_static_qris_payload, total_payable)?;
    let now = Utc::now();
    let short_hex = invoice_id
        .simple()
        .to_string()
        .chars()
        .rev()
        .take(5)
        .collect::<String>()
        .to_ascii_uppercase();
    let invoice_number = format!("CL-INV-{}-{}", now.format("%Y%m%d"), short_hex);
    let expires_at = now + Duration::minutes(15);

    let saved = invoice_repo::insert_invoice(
        &state.pool,
        NewInvoiceParams {
            id: invoice_id,
            invoice_number,
            user_id: auth.user_id,
            plan_code: plan.plan_code,
            target_tier: plan.target_tier,
            duration_days: plan.duration_days,
            base_amount_idr: plan.base_price_idr,
            unique_code_idr: unique_code,
            total_payable_idr: total_payable,
            dynamic_qris_payload: dynamic_payload,
            merchant_name: meta.merchant_name,
            acquirer_code: meta.acquirer_code,
            expires_at,
            applied_referral_code: applied_code,
            referrer_user_id: referrer_id,
            buyer_discount_idr: discount_idr,
            affiliate_commission_bps: comm_bps,
            affiliate_commission_idr: comm_idr,
        },
    )
    .await?;

    Ok(ApiResponse::ok(
        "QRIS pembayaran Coffeelabs berhasil dibuat.",
        DynamicQrisInvoiceDto::from_row(saved, meta.nmid),
    ))
}

pub async fn get_invoice_status(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<DynamicQrisInvoiceDto>>> {
    let row = invoice_repo::find_invoice_by_id(&state.pool, invoice_id)
        .await?
        .ok_or(QrisBillingError::InvoiceNotFound)?;
    if row.user_id != auth.user_id && !auth.claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::Forbidden(
            "Anda tidak memiliki akses ke invoice ini.".into(),
        ));
    }

    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);
    let mut dto = DynamicQrisInvoiceDto::from_row(row.clone(), meta.nmid);

    if row.status.eq_ignore_ascii_case("PAID") {
        if let Some(u) = subscription_repo::fetch_user_for_token(&state.pool, row.user_id).await? {
            if let Ok(tok) = state
                .jwt
                .generate_user_token(u.id, &u.username, &u.role, &u.tier, &u.status)
            {
                dto.fresh_access_token = Some(tok);
            }
            dto.subscription_expires_at = u.subscription_expires_at;
        }
    }

    Ok(ApiResponse::ok("Status invoice QRIS diperoleh.", dto))
}
