use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("Resep tidak ditemukan di penyimpanan cloud.")]
    NotFound,
}

impl From<RecipeError> for AppError {
    fn from(err: RecipeError) -> Self {
        match err {
            RecipeError::NotFound => AppError::NotFound(err.to_string()),
        }
    }
}
