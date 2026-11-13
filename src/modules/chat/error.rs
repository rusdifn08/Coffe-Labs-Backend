use thiserror::Error;

use crate::core::error::AppError;

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("{0}")]
    RateLimited(String),

    #[error("{0}")]
    ProfanityRejected(String),

    #[error("Anda bukan anggota ruang percakapan ini.")]
    NotParticipant,

    #[error("Ruang percakapan tidak ditemukan.")]
    ConversationNotFound,

    #[error("{0}")]
    InvalidFrame(String),

    #[error("Kesalahan database chat: {0}")]
    Storage(#[from] sqlx::Error),

    #[error("Kesalahan internal chat: {0}")]
    Internal(String),
}

impl ChatError {
    pub fn frame_error_code(&self) -> &'static str {
        match self {
            ChatError::RateLimited(_) => "RATE_LIMITED",
            ChatError::ProfanityRejected(_) => "PROFANITY_DETECTED",
            ChatError::NotParticipant => "FORBIDDEN_ROOM",
            ChatError::ConversationNotFound => "NOT_FOUND",
            ChatError::InvalidFrame(_) => "INVALID_FRAME",
            ChatError::Storage(_) => "STORAGE_ERROR",
            ChatError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl From<ChatError> for AppError {
    fn from(err: ChatError) -> Self {
        match err {
            ChatError::RateLimited(msg) => AppError::RateLimited(msg),
            ChatError::ProfanityRejected(msg) | ChatError::InvalidFrame(msg) => {
                AppError::Validation(msg)
            }
            ChatError::NotParticipant => AppError::Forbidden(err.to_string()),
            ChatError::ConversationNotFound => AppError::NotFound(err.to_string()),
            ChatError::Storage(sql_err) => AppError::Database(sql_err),
            ChatError::Internal(msg) => AppError::Internal(msg),
        }
    }
}
