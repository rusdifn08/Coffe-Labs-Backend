use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum SocialGraphError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    SelfFollowNotAllowed,
    TooManyRequests {
        message: String,
        retry_after_secs: u64,
    },
    Database(sqlx::Error),
    Internal(String),
}

impl From<sqlx::Error> for SocialGraphError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

impl IntoResponse for SocialGraphError {
    fn into_response(self) -> Response {
        match self {
            Self::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": msg, "code": "BAD_REQUEST" })),
            )
                .into_response(),
            Self::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": msg, "code": "UNAUTHORIZED" })),
            )
                .into_response(),
            Self::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": msg, "code": "FORBIDDEN" })),
            )
                .into_response(),
            Self::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": msg, "code": "NOT_FOUND" })),
            )
                .into_response(),
            Self::SelfFollowNotAllowed => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Tidak dapat mengikuti akun Anda sendiri.",
                    "code": "SELF_FOLLOW_NOT_ALLOWED"
                })),
            )
                .into_response(),
            Self::TooManyRequests {
                message,
                retry_after_secs,
            } => {
                let wait = retry_after_secs.max(1);
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, wait.to_string())],
                    Json(json!({
                        "error": message,
                        "code": "RATE_LIMITED",
                        "retry_after_secs": wait
                    })),
                )
                    .into_response()
            }
            Self::Database(err) => {
                tracing::error!(error = ?err, "SocialGraph database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Terjadi gangguan pada basis data grafik sosial.",
                        "code": "DATABASE_ERROR"
                    })),
                )
                    .into_response()
            }
            Self::Internal(msg) => {
                tracing::error!(error = %msg, "SocialGraph internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Terjadi kesalahan internal pada layanan grafik sosial.",
                        "code": "INTERNAL_ERROR"
                    })),
                )
                    .into_response()
            }
        }
    }
}
