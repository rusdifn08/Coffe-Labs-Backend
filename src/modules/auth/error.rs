use thiserror::Error;
use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum AuthDomainError {
    #[error("Username atau email dan password tidak sesuai.")]
    InvalidCredentials,

    #[error("Username '{0}' sudah digunakan. Silakan pilih username lain.")]
    UsernameTaken(String),

    #[error("Email '{0}' sudah terdaftar pada akun lain. Silakan login atau gunakan email lain.")]
    EmailTaken(String),

    #[error("Format alamat email tidak valid. Contoh: nama@gmail.com")]
    InvalidEmail,

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
            AuthDomainError::EmailTaken(_) => AppError::Conflict(err.to_string()),
            AuthDomainError::InvalidEmail => AppError::Validation(err.to_string()),
            AuthDomainError::TokenReuseDetected => AppError::Unauthorized(err.to_string()),
            AuthDomainError::InvalidRefreshToken => AppError::Unauthorized(err.to_string()),
            AuthDomainError::CurrentPasswordMismatch => AppError::BadRequest(err.to_string()),
        }
    }
}
