use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde_json::json;
use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum AdminControlError {
    #[error("Akses ditolak: hanya akun Admin yang berwenang mengubah kontrol sistem.")]
    AdminRoleRequired,

    #[error("Kartu fitur '{0}' tidak ditemukan di registri Barista Toolkit.")]
    FeatureNotFound(String),

    #[error("Status kunci tidak valid: '{0}'. Gunakan UNLOCKED, MAINTENANCE, COMING_SOON, atau UPDATED.")]
    InvalidLockStatus(String),

    #[error("Paket langganan '{0}' tidak ditemukan di katalog harga.")]
    PlanNotFound(String),

    #[error("{0}")]
    InvalidPriceConfig(String),

    #[error("Fitur sedang dikunci oleh Admin.")]
    FeatureLocked {
        feature_key: String,
        display_name: String,
        lock_status: String,
        title: String,
        message: String,
        estimated_unlock_at: Option<DateTime<Utc>>,
    },
}

impl From<AdminControlError> for AppError {
    fn from(err: AdminControlError) -> Self {
        match err {
            AdminControlError::AdminRoleRequired => AppError::Forbidden(err.to_string()),
            AdminControlError::FeatureNotFound(_) | AdminControlError::PlanNotFound(_) => {
                AppError::NotFound(err.to_string())
            }
            AdminControlError::InvalidLockStatus(_)
            | AdminControlError::InvalidPriceConfig(_) => AppError::Validation(err.to_string()),
            AdminControlError::FeatureLocked { message, .. } => AppError::Forbidden(message),
        }
    }
}

impl IntoResponse for AdminControlError {
    fn into_response(self) -> Response {
        match self {
            AdminControlError::FeatureLocked {
                feature_key,
                display_name,
                lock_status,
                title,
                message,
                estimated_unlock_at,
            } => (
                StatusCode::LOCKED,
                Json(json!({
                    "success": false,
                    "code": "FEATURE_LOCKED",
                    "error_code": "FEATURE_LOCKED",
                    "feature_key": feature_key,
                    "display_name": display_name,
                    "lock_status": lock_status,
                    "title": title,
                    "message": message,
                    "estimated_unlock_at": estimated_unlock_at
                })),
            )
                .into_response(),
            other => AppError::from(other).into_response(),
        }
    }
}
