use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct RecipeSyncQuery {
    pub username: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FavoriteToggleResponse {
    pub recipe_local_id: i64,
    pub is_favorite: bool,
}
