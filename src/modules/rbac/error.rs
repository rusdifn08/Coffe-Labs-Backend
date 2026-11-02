use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum RbacError {
    #[error("Kuota seduh Trial (10 seduhan) telah habis. Akun terkunci selama 7 hari atau silakan upgrade ke Coffee Labs Pro.")]
    TrialBrewQuotaLocked,

    #[error("Paket langganan '{0}' tidak dikenali.")]
    InvalidPlan(String),
}

impl From<RbacError> for AppError {
    fn from(err: RbacError) -> Self {
        match err {
            RbacError::TrialBrewQuotaLocked => AppError::QuotaExceeded(err.to_string()),
            RbacError::InvalidPlan(_) => AppError::BadRequest(err.to_string()),
        }
    }
}
