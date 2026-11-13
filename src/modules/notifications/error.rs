use axum::http::StatusCode;
use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum NotifError {
    #[error("Payload notifikasi tidak valid: {0}")]
    InvalidPayload(String),

    #[error("Token push perangkat sudah tidak terdaftar di FCM/APNs")]
    UnregisteredToken,

    #[error("Pengiriman sementara gagal (akan dicoba ulang): {reason}")]
    RetryableDelivery {
        retry_after_secs: Option<u64>,
        reason: String,
    },

    #[error("FCM HTTP error ({status}): {body}")]
    FcmHttp { status: u16, body: String },

    #[error("Kesalahan konfigurasi layanan push: {0}")]
    Config(String),

    #[error("Kesalahan database notifikasi: {0}")]
    Database(String),

    #[error("Kesalahan antrean Redis notifikasi: {0}")]
    Redis(String),
}

pub type NotifResult<T> = Result<T, NotifError>;

impl From<NotifError> for AppError {
    fn from(err: NotifError) -> Self {
        match err {
            NotifError::InvalidPayload(msg) => AppError::BadRequest(msg),
            NotifError::UnregisteredToken => AppError::NotFound(err.to_string()),
            NotifError::RetryableDelivery { .. } | NotifError::FcmHttp { .. } => {
                AppError::Internal(err.to_string())
            }
            NotifError::Config(msg) | NotifError::Database(msg) | NotifError::Redis(msg) => {
                AppError::Internal(msg)
            }
        }
    }
}

impl From<sqlx::Error> for NotifError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                NotifError::InvalidPayload("Notifikasi tidak ditemukan.".to_string())
            }
            other => NotifError::Database(other.to_string()),
        }
    }
}

pub fn http_status_is_retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}
