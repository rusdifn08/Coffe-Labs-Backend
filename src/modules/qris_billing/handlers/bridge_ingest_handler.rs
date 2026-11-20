use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

use crate::core::{
    auth_extractor::AuthenticatedUser,
    error::{ApiResponse, AppError, AppResult},
    AppState,
};
use crate::modules::qris_billing::{
    config::QrisBillingConfig,
    dto::{
        BridgeIngestPayloadDto, BridgeIngestResponseDto, DynamicQrisInvoiceDto,
        ManualProofUploadDto,
    },
    emvco_qris::extract_merchant_meta,
    error::QrisBillingError,
    repositories::invoice_repo,
    services::{
        bridge_security_guard::{parse_indonesian_rupiah_amount, verify_bridge_request},
        reconcile_upgrade_service::{reconcile_incoming_payment, IncomingMutationInput},
    },
};

/// Channel A: Endpoint penerimaan notifikasi Livin' Merchant dari HP Admin (`id.bmri.livinmerchant`).
/// Dilindungi Zero-Trust: `X-Bridge-Timestamp` (<= 60s), `X-Bridge-Nonce` (Anti-Replay), & `X-Bridge-Signature` (HMAC-SHA256 Constant-Time).
pub async fn ingest_livin_bridge_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    raw_body: Bytes,
) -> AppResult<Json<ApiResponse<BridgeIngestResponseDto>>> {
    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let ts_hdr = headers
        .get("x-bridge-timestamp")
        .and_then(|v| v.to_str().ok());
    let nonce_hdr = headers.get("x-bridge-nonce").and_then(|v| v.to_str().ok());
    let sig_hdr = headers
        .get("x-bridge-signature")
        .and_then(|v| v.to_str().ok());

    let redis_pool = state.redis.active_pool();
    verify_bridge_request(
        redis_pool.as_ref(),
        &cfg.bridge_hmac_secret,
        ts_hdr,
        nonce_hdr,
        sig_hdr,
        &raw_body,
    )
    .await?;

    let payload: BridgeIngestPayloadDto = serde_json::from_slice(&raw_body)
        .map_err(|_| AppError::BadRequest("Payload JSON Livin' Merchant Bridge tidak valid.".into()))?;

    if let Some(pkg) = payload.package_name.as_deref() {
        let trimmed = pkg.trim();
        if !trimmed.is_empty() && trimmed != "id.bmri.livinmerchant" {
            return Err(AppError::Forbidden(
                "Hanya notifikasi dari paket resmi id.bmri.livinmerchant yang diterima.".into(),
            ));
        }
    }

    let combined_text = match payload.title.as_deref() {
        Some(t) if !t.trim().is_empty() => format!("{} — {}", t.trim(), payload.raw_notification_text.trim()),
        _ => payload.raw_notification_text.trim().to_string(),
    };

    let amount_idr = payload
        .amount_idr
        .filter(|&v| v > 0)
        .or_else(|| parse_indonesian_rupiah_amount(&combined_text))
        .ok_or_else(|| {
            AppError::BadRequest(
                "Nominal Rupiah tidak ditemukan pada notifikasi Livin' Merchant.".into(),
            )
        })?;

    let received_at = payload
        .received_at_ms
        .and_then(|ms| Utc.timestamp_millis_opt(ms).single())
        .unwrap_or_else(Utc::now);

    let result = reconcile_incoming_payment(
        &state,
        IncomingMutationInput {
            source_channel: "LIVIN_ANDROID_BRIDGE".to_string(),
            amount_idr,
            payer_name: payload.payer_name,
            bank_rrn_ref: payload.bank_rrn_ref,
            raw_notification_text: combined_text,
            received_at,
        },
    )
    .await?;

    Ok(ApiResponse::ok(
        "Notifikasi pembayaran Livin' Merchant berhasil diproses.",
        result,
    ))
}

/// Layer 3 Fallback: Pengguna mengunggah URL bukti transfer Cloudinary jika membayar melewati batas waktu.
pub async fn upload_manual_proof(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(invoice_id): Path<Uuid>,
    Json(req): Json<ManualProofUploadDto>,
) -> AppResult<Json<ApiResponse<DynamicQrisInvoiceDto>>> {
    let url = req.proof_cloudinary_url.trim();
    if url.is_empty() || (!url.starts_with("https://") && !url.starts_with("http://")) {
        return Err(AppError::BadRequest("URL bukti transfer Cloudinary tidak valid.".into()));
    }

    let updated = invoice_repo::attach_manual_proof(&state.pool, invoice_id, auth.user_id, url)
        .await?
        .ok_or(QrisBillingError::InvoiceNotFound)?;

    let cfg = QrisBillingConfig::from_app_config(&state.config);
    let meta = extract_merchant_meta(&cfg.master_static_qris_payload);

    Ok(ApiResponse::ok(
        "Bukti transfer berhasil diunggah. Tim Coffee Labs akan segera memverifikasi.",
        DynamicQrisInvoiceDto::from_row(updated, meta.nmid),
    ))
}
