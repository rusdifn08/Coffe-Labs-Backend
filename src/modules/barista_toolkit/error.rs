use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum BaristaToolkitError {
    #[error("Fitur ini memerlukan lisensi akun tier {0} atau lebih tinggi.")]
    TierUpgradeRequired(String),
    #[error("Materi atau E-Book yang diminta tidak ditemukan.")]
    NotFound,
    #[error("Chapter Game Brew ini belum terbuka. Selesaikan chapter sebelumnya terlebih dahulu.")]
    ChapterLocked,
    #[error("Kunci dekripsi atau paket lisensi E-Book tidak valid: {0}")]
    InvalidBookLicense(String),
    #[error("Parameter masukan tidak valid: {0}")]
    Validation(String),
    #[error("Kesalahan database Barista Toolkit: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<BaristaToolkitError> for AppError {
    fn from(err: BaristaToolkitError) -> Self {
        match err {
            BaristaToolkitError::TierUpgradeRequired(_) | BaristaToolkitError::ChapterLocked => {
                AppError::Forbidden(err.to_string())
            }
            BaristaToolkitError::NotFound => AppError::NotFound(err.to_string()),
            BaristaToolkitError::InvalidBookLicense(_) => AppError::BadRequest(err.to_string()),
            BaristaToolkitError::Validation(msg) => AppError::Validation(msg),
            BaristaToolkitError::Database(e) => AppError::Database(e),
        }
    }
}
