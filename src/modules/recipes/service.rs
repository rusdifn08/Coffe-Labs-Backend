use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, security::Claims},
    modules::recipes::{
        dto::{FavoriteToggleResponse, RecipeSyncQuery},
        models::UserRecipeRow,
        repository,
    },
};

pub async fn sync_recipes(
    pool: &PgPool,
    auth: Option<Claims>,
    q: RecipeSyncQuery,
) -> AppResult<Vec<serde_json::Value>> {
    let username = auth
        .map(|a| a.username)
        .or(q.username)
        .unwrap_or_default();
    let rows = repository::list_user_recipes(pool, &username, q.category.as_deref()).await?;
    Ok(rows.into_iter().map(|r| r.recipe_data).collect())
}

pub async fn save_recipe(
    pool: &PgPool,
    auth: Option<Claims>,
    payload: serde_json::Value,
) -> AppResult<UserRecipeRow> {
    let user_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let username = auth
        .map(|c| c.username)
        .or_else(|| {
            payload
                .get("author")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "guest".to_string());

    let local_id = payload
        .get("id")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Custom Brew Recipe")
        .to_string();
    let method = payload
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("Hario V60 02")
        .to_string();
    let category = payload
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("V60")
        .to_string();

    repository::upsert_recipe(
        pool,
        user_id,
        &username,
        local_id,
        &title,
        &method,
        &category,
        payload,
    )
    .await
}

pub async fn remove_recipe(pool: &PgPool, auth: Option<Claims>, local_id: i64) -> AppResult<()> {
    let username = auth.map(|c| c.username).unwrap_or_else(|| "guest".to_string());
    let _ = repository::delete_by_local_id(pool, &username, local_id).await?;
    Ok(())
}

pub async fn toggle_recipe_favorite(
    pool: &PgPool,
    auth: Option<Claims>,
    local_id: i64,
) -> AppResult<FavoriteToggleResponse> {
    let username = auth.map(|c| c.username).unwrap_or_else(|| "guest".to_string());
    let is_favorite = repository::toggle_favorite(pool, &username, local_id).await?;
    Ok(FavoriteToggleResponse {
        recipe_local_id: local_id,
        is_favorite,
    })
}
