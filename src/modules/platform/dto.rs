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
