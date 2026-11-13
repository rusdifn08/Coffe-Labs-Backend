use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

fn default_media_folder() -> String {
    "coffe_labs/media".to_string()
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UploadSignatureRequest {
    #[serde(default = "default_media_folder")]
    #[validate(length(min = 1, max = 128, message = "Folder harus 1-128 karakter"))]
    pub folder: String,

    #[serde(default)]
    #[validate(length(max = 128, message = "public_id maksimal 128 karakter"))]
    pub public_id: Option<String>,
}
