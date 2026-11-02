use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::PgPool;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::OptionalAuthUser,
    },
    modules::recipes::{dto::RecipeSyncQuery, service},
};

pub async fn get_synced_recipes(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<RecipeSyncQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let list = service::sync_recipes(&pool, auth, query).await?;
    Ok(Json(list))
}

pub async fn upsert_custom_recipe(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<impl IntoResponse> {
    let saved = service::save_recipe(&pool, auth, payload).await?;
    Ok(ApiResponse::created("Resep custom berhasil disinkronkan ke cloud.", saved))
}

pub async fn delete_custom_recipe(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    service::remove_recipe(&pool, auth, id).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Resep custom berhasil dihapus dari cloud."
    })))
}

pub async fn toggle_favorite_recipe(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let res = service::toggle_recipe_favorite(&pool, auth, id).await?;
    Ok(ApiResponse::ok("Status favorit resep diperbarui.", res))
}
