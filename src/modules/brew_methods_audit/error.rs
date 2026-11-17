use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum BrewAuditError {
    #[error("Parameter metode seduh tidak valid: {0}")]
    Validation(String),

    #[error("Alarm Cold Brew tidak ditemukan.")]
    AlarmNotFound,

    #[error("Akses alarm ditolak.")]
    Forbidden,

    #[error("Kegagalan database: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<BrewAuditError> for AppError {
    fn from(value: BrewAuditError) -> Self {
        match value {
            BrewAuditError::Validation(msg) => AppError::Validation(msg),
            BrewAuditError::AlarmNotFound => {
                AppError::NotFound("Alarm Cold Brew tidak ditemukan.".to_string())
            }
            BrewAuditError::Forbidden => {
                AppError::Forbidden("Akses alarm tidak diizinkan.".to_string())
            }
            BrewAuditError::Database(err) => AppError::Database(err),
        }
    }
}
