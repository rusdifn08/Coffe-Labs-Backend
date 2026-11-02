use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::auth::models::UserSummary;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50, message = "Username minimal harus 3 karakter dan maksimal 50 karakter."))]
    pub username: String,
    #[validate(length(min = 4, max = 128, message = "Password minimal harus 4 karakter."))]
    pub password: String,
    #[validate(length(min = 1, max = 100, message = "Nama lengkap tidak boleh kosong."))]
    pub full_name: String,
    pub avatar_url: Option<String>,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 50, message = "Username tidak boleh kosong."))]
    pub username: String,
    #[validate(length(min = 1, max = 128, message = "Password tidak boleh kosong."))]
    pub password: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequest {
    #[validate(length(min = 10, max = 256, message = "Refresh token tidak valid."))]
    pub refresh_token: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, max = 128, message = "Password saat ini diperlukan."))]
    pub current_password: String,
    #[validate(length(min = 4, max = 128, message = "Password baru minimal 4 karakter."))]
    pub new_password: String,
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct AuthSessionPayload {
    pub token: String,
    pub refresh_token: String,
    pub user: UserSummary,
    pub message: String,
}

#[derive(Debug, Serialize, Clone, ToSchema)]
pub struct UnifiedAuthResponse {
    pub success: bool,
    pub message: String,
    pub token: String,
    pub refresh_token: String,
    pub user: UserSummary,
    pub data: AuthSessionPayload,
}
