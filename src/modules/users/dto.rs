use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1, max = 100, message = "Nama lengkap minimal 1 karakter."))]
    pub full_name: Option<String>,
    #[validate(length(max = 160, message = "Alamat email terlalu panjang."))]
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub equipment_json: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateUserTierRequest {
    #[validate(length(min = 3, max = 30, message = "Tier harus berupa 'trial', 'trial_7d', 'pro', atau 'business'."))]
    pub tier: String,
    pub trial_days: Option<i32>,
}
