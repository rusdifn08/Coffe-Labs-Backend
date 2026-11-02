use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum SocialError {
    #[error("Postingan komunitas tidak ditemukan.")]
    PostNotFound,

    #[error("Tidak dapat mengikuti akun sendiri.")]
    CannotFollowSelf,
}

impl From<SocialError> for AppError {
    fn from(err: SocialError) -> Self {
        match err {
            SocialError::PostNotFound => AppError::NotFound(err.to_string()),
            SocialError::CannotFollowSelf => AppError::BadRequest(err.to_string()),
        }
    }
}
