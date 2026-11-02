use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum BrewLogError {
    #[error("Catatan jurnal seduh tidak ditemukan.")]
    NotFound,
}

impl From<BrewLogError> for AppError {
    fn from(err: BrewLogError) -> Self {
        match err {
            BrewLogError::NotFound => AppError::NotFound(err.to_string()),
        }
    }
}
