use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterTrainedModelRequest {
    pub class_slug: String,
    pub roastery_name: String,
    pub bean_name: String,
    pub varietal: Option<String>,
    pub process: Option<String>,
    pub tasting_notes: Option<Vec<String>>,
    pub altitude_masl: Option<String>,
    pub recommended_recipe_id: Option<String>,
    pub weights_vector: Vec<f32>,
    pub sub_centroids: Option<Vec<Vec<f32>>>,
    pub cosine_threshold: Option<f32>,
    pub augmented_samples_count: Option<i32>,
    pub validation_accuracy: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisterTrainedModelResponse {
    pub success: bool,
    pub class_slug: String,
    pub message: String,
    pub dimensions: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TrainedModelSummaryDto {
    pub id: uuid::Uuid,
    pub class_slug: String,
    pub roastery_name: String,
    pub bean_name: String,
    pub varietal: String,
    pub process: String,
    pub tasting_notes: Vec<String>,
    pub altitude_masl: Option<String>,
    pub recommended_recipe_id: Option<String>,
    pub cosine_threshold: f32,
    pub augmented_samples_count: i32,
    pub validation_accuracy: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
