use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum UsersError {
    #[error("User tidak ditemukan atau merupakan akun Admin yang dilindungi.")]
    UserNotFoundOrProtected,
}

impl From<UsersError> for AppError {
    fn from(err: UsersError) -> Self {
        match err {
            UsersError::UserNotFoundOrProtected => AppError::NotFound(err.to_string()),
        }
    }
}
