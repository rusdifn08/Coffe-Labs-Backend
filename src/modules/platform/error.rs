use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Feedback tidak ditemukan.")]
    FeedbackNotFound,
}

impl From<PlatformError> for AppError {
    fn from(err: PlatformError) -> Self {
        match err {
            PlatformError::FeedbackNotFound => AppError::NotFound(err.to_string()),
        }
    }
}
