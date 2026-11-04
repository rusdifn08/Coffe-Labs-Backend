#![allow(dead_code)]
use crate::core::error::AppError;

pub type BusinessResult<T> = Result<T, AppError>;

pub fn invalid_asset_error(msg: impl Into<String>) -> AppError {
    AppError::BadRequest(msg.into())
}

pub fn invalid_pin_error() -> AppError {
    AppError::Unauthorized("PIN Kasir yang Anda masukkan salah.".to_string())
}
