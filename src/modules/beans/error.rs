use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum BeansError {
    #[error("Data biji kopi tidak ditemukan di katalog.")]
    BeanNotFound,
}

impl From<BeansError> for AppError {
    fn from(err: BeansError) -> Self {
        match err {
            BeansError::BeanNotFound => AppError::NotFound(err.to_string()),
        }
    }
}
