use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1, max = 100, message = "Nama lengkap minimal 1 karakter."))]
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub equipment_json: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateUserTierRequest {
    #[validate(length(min = 3, max = 20, message = "Tier harus berupa 'trial', 'pro', atau 'business'."))]
    pub tier: String,
}
