use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum SocialFeedError {
    #[error("Profil barista tidak ditemukan.")]
    ProfileNotFound,

    #[error("Thread atau postingan tidak ditemukan.")]
    ThreadNotFound,

    #[error("Cursor paginasi tidak valid.")]
    InvalidCursor,

    #[error("Anda tidak dapat mengikuti akun Anda sendiri.")]
    SelfFollowForbidden,

    #[error("Parameter atau lampiran thread tidak valid: {0}")]
    BadRequest(String),

    #[error("Konten ditolak oleh sistem moderasi Social Coffee: {0}")]
    ModerationRejected(String),

    #[error("Anda tidak memiliki izin mengubah postingan ini.")]
    Forbidden,

    #[error("Kesalahan database: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<SocialFeedError> for AppError {
    fn from(err: SocialFeedError) -> Self {
        match err {
            SocialFeedError::ProfileNotFound | SocialFeedError::ThreadNotFound => {
                AppError::NotFound(err.to_string())
            }
            SocialFeedError::InvalidCursor
            | SocialFeedError::SelfFollowForbidden
            | SocialFeedError::BadRequest(_) => AppError::BadRequest(err.to_string()),
            SocialFeedError::ModerationRejected(m) => AppError::Validation(m),
            SocialFeedError::Forbidden => AppError::Forbidden(err.to_string()),
            SocialFeedError::Database(e) => AppError::Database(e),
        }
    }
}
