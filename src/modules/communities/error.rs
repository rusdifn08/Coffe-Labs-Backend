use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum CommunitiesError {
    #[error("Komunitas atau channel kopi tidak ditemukan.")]
    ClubNotFound,

    #[error("Kode Secret ID atau link undangan tidak valid, kedaluwarsa, atau sudah dicabut.")]
    InvalidOrExpiredSecret,

    #[error("Terlalu banyak percobaan Secret ID. Coba lagi dalam 1 menit.")]
    SecretRateLimited,

    #[error("Anda bukan anggota club kopi ini atau akses ditolak.")]
    Forbidden,

    #[error("Permintaan tidak valid: {0}")]
    BadRequest(String),

    #[error("Validasi gagal: {0}")]
    Validation(String),

    #[error("Handle atau slug komunitas sudah digunakan.")]
    Conflict(String),

    #[error("Kesalahan kriptografi internal.")]
    CryptoFailure,

    #[error("Kesalahan database: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<CommunitiesError> for AppError {
    fn from(err: CommunitiesError) -> Self {
        match err {
            CommunitiesError::ClubNotFound => AppError::NotFound(err.to_string()),
            CommunitiesError::InvalidOrExpiredSecret | CommunitiesError::BadRequest(_) => {
                AppError::BadRequest(err.to_string())
            }
            CommunitiesError::SecretRateLimited => AppError::RateLimited(err.to_string()),
            CommunitiesError::Forbidden => AppError::Forbidden(err.to_string()),
            CommunitiesError::Validation(m) => AppError::Validation(m),
            CommunitiesError::Conflict(m) => AppError::Conflict(m),
            CommunitiesError::CryptoFailure => AppError::Internal(err.to_string()),
            CommunitiesError::Database(e) => AppError::Database(e),
        }
    }
}
