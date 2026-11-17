use crate::core::error::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MonitoringError {
    #[error("Akses monitoring hanya tersedia untuk akun Admin atau Business.")]
    Forbidden,
    #[error("Payload batch telemetri terlalu besar atau melebihi batas dekompresi aman.")]
    PayloadTooLarge,
    #[error("Format gzip pada batch telemetri tidak valid.")]
    InvalidGzip,
    #[error("Format JSON batch telemetri tidak valid: {0}")]
    InvalidPayload(String),
    #[error("Dossier monitoring akun tidak ditemukan.")]
    AccountNotFound,
    #[error("Kesalahan database monitoring: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<MonitoringError> for AppError {
    fn from(err: MonitoringError) -> Self {
        match err {
            MonitoringError::Forbidden => AppError::Forbidden(err.to_string()),
            MonitoringError::PayloadTooLarge | MonitoringError::InvalidGzip => {
                AppError::BadRequest(err.to_string())
            }
            MonitoringError::InvalidPayload(msg) => AppError::Validation(msg),
            MonitoringError::AccountNotFound => AppError::NotFound(err.to_string()),
            MonitoringError::Database(e) => AppError::Database(e),
        }
    }
}
