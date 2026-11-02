use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum AuthDomainError {
    #[error("Username atau password salah.")]
    InvalidCredentials,

    #[error("Username '{0}' sudah digunakan. Silakan pilih username lain.")]
    UsernameTaken(String),

    #[error("Refresh token telah dicabut atau mendeteksi reuse mencurigakan. Seluruh sesi perangkat dihentikan.")]
    TokenReuseDetected,

    #[error("Refresh token tidak valid atau telah kedaluwarsa.")]
    InvalidRefreshToken,

    #[error("Password saat ini tidak sesuai.")]
    CurrentPasswordMismatch,
}

impl From<AuthDomainError> for AppError {
    fn from(err: AuthDomainError) -> Self {
        match err {
            AuthDomainError::InvalidCredentials => AppError::Unauthorized(err.to_string()),
            AuthDomainError::UsernameTaken(_) => AppError::Conflict(err.to_string()),
            AuthDomainError::TokenReuseDetected => AppError::Unauthorized(err.to_string()),
            AuthDomainError::InvalidRefreshToken => AppError::Unauthorized(err.to_string()),
            AuthDomainError::CurrentPasswordMismatch => AppError::BadRequest(err.to_string()),
        }
    }
}
