use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum MarketplaceError {
    #[error("Kode voucher '{0}' tidak ditemukan atau sudah tidak berlaku.")]
    InvalidVoucher(String),
}

impl From<MarketplaceError> for AppError {
    fn from(err: MarketplaceError) -> Self {
        match err {
            MarketplaceError::InvalidVoucher(_) => AppError::BadRequest(err.to_string()),
        }
    }
}
