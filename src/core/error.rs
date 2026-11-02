use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use tracing::{error, warn};
use utoipa::ToSchema;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(message: impl Into<String>, data: T) -> Json<Self> {
        Json(Self {
            success: true,
            message: message.into(),
            data: Some(data),
        })
    }

    pub fn created(message: impl Into<String>, data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::CREATED,
            Json(Self {
                success: true,
                message: message.into(),
                data: Some(data),
            }),
        )
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    Unauthorized(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("Account status: {status} - {message}")]
    AccountStatus { status: String, message: String },

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("{0}")]
    RateLimited(String),

    #[error("{0}")]
    QuotaExceeded(String),

    #[error("Database operation failed")]
    Database(#[from] sqlx::Error),

    #[error("Internal system error")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Validation(msg) | AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error_code": "BAD_REQUEST",
                    "message": msg
                })),
            )
                .into_response(),
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "error_code": "UNAUTHORIZED",
                    "message": msg
                })),
            )
                .into_response(),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error_code": "FORBIDDEN",
                    "message": msg
                })),
            )
                .into_response(),
            AppError::AccountStatus { status, message } => (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "error_code": "ACCOUNT_RESTRICTED",
                    "status": status,
                    "message": message
                })),
            )
                .into_response(),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error_code": "NOT_FOUND",
                    "message": msg
                })),
            )
                .into_response(),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "success": false,
                    "error_code": "CONFLICT",
                    "message": msg
                })),
            )
                .into_response(),
            AppError::RateLimited(msg) => {
                warn!("Rate limit triggered: {}", msg);
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "success": false,
                        "error_code": "RATE_LIMITED",
                        "message": msg
                    })),
                )
                    .into_response()
            }
            AppError::QuotaExceeded(msg) => (
                StatusCode::PAYMENT_REQUIRED,
                Json(json!({
                    "success": false,
                    "error_code": "QUOTA_EXCEEDED",
                    "message": msg
                })),
            )
                .into_response(),
            AppError::Database(db_err) => {
                error!("Sanitized NeonDB Error: {:?}", db_err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error_code": "DATABASE_ERROR",
                        "message": "Terjadi gangguan pemrosesan data pada server. Silakan coba sesaat lagi."
                    })),
                )
                    .into_response()
            }
            AppError::Internal(internal_msg) => {
                error!("Sanitized Internal Error: {}", internal_msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error_code": "INTERNAL_ERROR",
                        "message": "Terjadi gangguan sistem internal. Tim teknis telah diberitahu."
                    })),
                )
                    .into_response()
            }
        }
    }
}
