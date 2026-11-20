use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::{
    auth_extractor::AuthenticatedUser,
    error::{ApiResponse, AppResult},
    AppState,
};
use crate::modules::qris_billing::{
    config::QrisBillingConfig,
    dto::{
        AdminTestQrisDto, AdminTestQrisResponseDto, DynamicQrisInvoiceDto, UnmatchedMutationDto,
    },
    emvco_qris::{build_dynamic_qris, extract_merchant_meta, verify_qris_crc16},
    error::QrisBillingError,
    repositories::invoice_repo,
    services::reconcile_upgrade_service::admin_manual_approve_invoice,
};

fn require_admin(auth: &AuthenticatedUser) -> AppResult<()> {
    if !auth.claims.role.eq_ignore_ascii_case("admin") {
        return Err(QrisBillingError::AdminOnly.into());
    }
    Ok(())
}

pub async fn list_unmatched_and_recent_mutations(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<UnmatchedMutationDto>>>> {
    require_admin(&auth)?;
    let rows = invoice_repo::list_recent_mutations(&state.pool, 60).await?;
    let dtos = rows.into_iter().map(UnmatchedMutationDto::from).collect();
    Ok(ApiResponse::ok("Daftar mutasi masuk Livin' Merchant.", dtos))
}

pub async fn list_pending_and_manual_invoices(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<DynamicQrisInvoiceDto>>>> {
    require_admin(&auth)?;
    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);
    let rows = invoice_repo::list_manual_review_invoices(&state.pool).await?;
    let dtos = rows
        .into_iter()
        .map(|r| DynamicQrisInvoiceDto::from_row(r, meta.nmid.clone()))
        .collect();
    Ok(ApiResponse::ok("Daftar invoice menunggu pembayaran / tinjauan manual.", dtos))
}

pub async fn manual_approve_invoice(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<DynamicQrisInvoiceDto>>> {
    require_admin(&auth)?;
    let (invoice, upgraded, fresh_token) =
        admin_manual_approve_invoice(&state, invoice_id, &auth.claims.username).await?;

    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);
    let mut dto = DynamicQrisInvoiceDto::from_row(invoice, meta.nmid);
    dto.fresh_access_token = Some(fresh_token);
    dto.subscription_expires_at = upgraded.subscription_expires_at;

    Ok(ApiResponse::ok(
        format!(
            "Invoice {} disetujui! Akun @{} kini aktif pada tier {}.",
            dto.invoice_number, upgraded.username, upgraded.tier
        ),
        dto,
    ))
}

pub async fn admin_test_dynamic_qris(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<AdminTestQrisDto>,
) -> AppResult<Json<ApiResponse<AdminTestQrisResponseDto>>> {
    require_admin(&auth)?;
    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);
    let dynamic_payload = build_dynamic_qris(&cfg.master_static_qris_payload, req.amount_idr)?;
    let crc_valid = verify_qris_crc16(&dynamic_payload)?;

    Ok(ApiResponse::ok(
        "Uji generasi EMVCo QRIS Dinamis & CRC-16 berhasil.",
        AdminTestQrisResponseDto {
            merchant_name: meta.merchant_name,
            merchant_city: meta.merchant_city,
            acquirer_code: meta.acquirer_code,
            nmid: meta.nmid,
            amount_idr: req.amount_idr,
            dynamic_qris_payload: dynamic_payload,
            crc_valid,
        },
    ))
}

/// Mengembalikan metadata konfigurasi Bridge & Merchant (tanpa membocorkan raw secret kecuali diminta admin aktif).
pub async fn get_admin_bridge_info(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Value>>> {
    require_admin(&auth)?;
    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);
    let static_crc_valid = verify_qris_crc16(&cfg.master_static_qris_payload).unwrap_or(false);

    Ok(ApiResponse::ok(
        "Konfigurasi Merchant QRIS Bridge Coffeelabs.",
        json!({
            "merchant_name": meta.merchant_name,
            "merchant_city": meta.merchant_city,
            "acquirer_code": meta.acquirer_code,
            "nmid": meta.nmid,
            "static_qris_crc_valid": static_crc_valid,
            "bridge_hmac_secret": cfg.bridge_hmac_secret,
            "imap_watcher_configured": cfg.imap.is_some(),
        }),
    ))
}
