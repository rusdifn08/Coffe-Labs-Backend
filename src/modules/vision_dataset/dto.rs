use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionClassItemDto {
    pub class_slug: String,
    pub brand: String,
    pub coffee_name: String,
    pub process: String,
    pub tasting_notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centroid_vector: Option<Vec<f32>>,
    pub verified: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncVisionClassRequest {
    pub class_slug: String,
    pub brand: String,
    pub coffee_name: String,
    pub process: String,
    #[serde(default)]
    pub tasting_notes: Vec<String>,
    pub centroid_vector: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncVisionClassResponse {
    pub success: bool,
    pub message: String,
    pub class_slug: String,
}
