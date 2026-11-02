use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("Gagal mengambil metrik sistem.")]
    CollectionFailed,
}

impl From<TelemetryError> for AppError {
    fn from(err: TelemetryError) -> Self {
        match err {
            TelemetryError::CollectionFailed => AppError::Internal(err.to_string()),
        }
    }
}
