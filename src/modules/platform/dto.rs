use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::modules::platform::models::UpdateAction;

#[derive(Debug, Deserialize, Default, ToSchema)]
#[allow(dead_code)]
pub struct VersionCheckQuery {
    pub platform: Option<String>,
    pub base_version: Option<String>,
    pub build_number: Option<i32>,
    pub patch_number: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VersionCheckResponse {
    pub action: UpdateAction,
    pub action_code: String,
    pub target_patch: Option<i32>,
    pub latest_base_version: String,
    pub latest_build_number: i32,
    pub is_force_update: bool,
    pub release_notes: String,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateAppVersionConfigRequest {
    pub min_base_version: Option<String>,
    pub latest_base_version: Option<String>,
    pub min_build_number: Option<i32>,
    pub latest_build_number: Option<i32>,
    pub latest_patch_number: Option<i32>,
    pub is_force_update: Option<bool>,
    pub is_patch_required: Option<bool>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateFeedbackRequest {
    #[validate(length(min = 1, max = 50, message = "Kategori feedback diperlukan."))]
    pub category: String,
    #[validate(length(min = 1, max = 100, message = "Fitur yang terdampak diperlukan."))]
    pub feature_affected: String,
    #[validate(length(min = 1, max = 250, message = "Judul feedback tidak boleh kosong."))]
    pub title: String,
    #[validate(length(min = 1, max = 4000, message = "Deskripsi feedback tidak boleh kosong."))]
    pub description: String,
    pub rating: Option<i32>,
    pub app_version: Option<String>,
    pub device_info: Option<String>,
    pub guest_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateFeedbackStatusRequest {
    #[validate(length(min = 1, max = 30))]
    pub status: String,
    pub admin_notes: Option<String>,
}

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct FeedbackQuery {
    pub status: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateTrialPolicyRequest {
    pub policy_mode: Option<String>,
    pub trial_duration_days: Option<i32>,
    pub unlock_all_recipes: Option<bool>,
    pub unlock_smart_brew_dial_in: Option<bool>,
    pub unlock_ai_scanner: Option<bool>,
    pub unlock_toolkit_pro: Option<bool>,
    pub unlock_learning_academy: Option<bool>,
    pub unlock_brew_doctor: Option<bool>,
    pub unlimited_brews: Option<bool>,
    pub max_free_recipes: Option<i32>,
    pub max_trial_brews: Option<i32>,
    pub allow_business_suite: Option<bool>,
    pub apply_to_guests: Option<bool>,
    pub apply_to_registered_trial: Option<bool>,
    pub policy_note: Option<String>,
    pub reset_activated_at: Option<bool>,
}
