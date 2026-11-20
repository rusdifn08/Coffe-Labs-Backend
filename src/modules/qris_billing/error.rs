use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum QrisBillingError {
    #[error("Signature HMAC-SHA256 bridge tidak valid.")]
    BridgeSignatureInvalid,

    #[error("Timestamp bridge kedaluwarsa atau melampaui batas drift 60 detik.")]
    BridgeTimestampExpired,

    #[error("Nonce bridge sudah pernah digunakan (replay ditolak).")]
    BridgeReplayDetected,

    #[error("Format string QRIS EMVCo tidak valid: {0}")]
    InvalidQrisPayload(String),

    #[error("Nominal QRIS Rp{0} di luar rentang standar Bank Indonesia (Rp1 - Rp10.000.000).")]
    AmountOutOfRange(i64),

    #[error("Kode paket langganan tidak dikenali: {0}")]
    UnknownPlanCode(String),

    #[error("Seluruh slot kode unik (Rp1 - Rp999) sedang penuh sementara. Coba lagi dalam 1 menit.")]
    AmountPoolExhausted,

    #[error("Invoice QRIS tidak ditemukan.")]
    InvoiceNotFound,

    #[error("Invoice QRIS telah kedaluwarsa. Silakan buat QRIS baru.")]
    InvoiceExpired,

    #[error("Hanya superadmin yang dapat mengakses endpoint rekonsiliasi QRIS.")]
    AdminOnly,
}

impl From<QrisBillingError> for AppError {
    fn from(err: QrisBillingError) -> Self {
        match err {
            QrisBillingError::BridgeSignatureInvalid
            | QrisBillingError::BridgeTimestampExpired
            | QrisBillingError::BridgeReplayDetected => AppError::Unauthorized(err.to_string()),
            QrisBillingError::InvalidQrisPayload(_)
            | QrisBillingError::AmountOutOfRange(_)
            | QrisBillingError::UnknownPlanCode(_) => AppError::BadRequest(err.to_string()),
            QrisBillingError::AmountPoolExhausted | QrisBillingError::InvoiceExpired => {
                AppError::Conflict(err.to_string())
            }
            QrisBillingError::InvoiceNotFound => AppError::NotFound(err.to_string()),
            QrisBillingError::AdminOnly => AppError::Forbidden(err.to_string()),
        }
    }
}
