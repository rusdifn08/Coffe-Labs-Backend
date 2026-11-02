use sqlx::PgPool;
use uuid::Uuid;

use crate::{core::error::AppResult, modules::recipes::models::UserRecipeRow};

pub async fn list_user_recipes(
    pool: &PgPool,
    username: &str,
    category: Option<&str>,
) -> AppResult<Vec<UserRecipeRow>> {
    let rows = sqlx::query_as::<_, UserRecipeRow>(
        r#"SELECT * FROM user_recipes
           WHERE ($1::text = '' OR username = $1)
             AND ($2::text IS NULL OR category ILIKE $2)
           ORDER BY updated_at DESC
           LIMIT 200"#,
    )
    .bind(username)
    .bind(category)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn upsert_recipe(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    recipe_local_id: i64,
    title: &str,
    method: &str,
    category: &str,
    recipe_data: serde_json::Value,
) -> AppResult<UserRecipeRow> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM user_recipes WHERE username = $1 AND recipe_local_id = $2 LIMIT 1",
    )
    .bind(username)
    .bind(recipe_local_id)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = existing {
        let updated = sqlx::query_as::<_, UserRecipeRow>(
            r#"UPDATE user_recipes
               SET title = $1, method = $2, category = $3, recipe_data = $4, updated_at = NOW()
               WHERE id = $5
               RETURNING *"#,
        )
        .bind(title)
        .bind(method)
        .bind(category)
        .bind(recipe_data)
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(updated)
    } else {
        let inserted = sqlx::query_as::<_, UserRecipeRow>(
            r#"INSERT INTO user_recipes (
                user_id, username, recipe_local_id, title, method, category, recipe_data
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *"#,
        )
        .bind(user_id)
        .bind(username)
        .bind(recipe_local_id)
        .bind(title)
        .bind(method)
        .bind(category)
        .bind(recipe_data)
        .fetch_one(pool)
        .await?;
        Ok(inserted)
    }
}

pub async fn delete_by_local_id(
    pool: &PgPool,
    username: &str,
    recipe_local_id: i64,
) -> AppResult<u64> {
    let res = sqlx::query("DELETE FROM user_recipes WHERE username = $1 AND recipe_local_id = $2")
        .bind(username)
        .bind(recipe_local_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn toggle_favorite(
    pool: &PgPool,
    username: &str,
    recipe_local_id: i64,
) -> AppResult<bool> {
    let fav: Option<bool> = sqlx::query_scalar(
        "UPDATE user_recipes SET is_favorite = NOT is_favorite, updated_at = NOW() WHERE username = $1 AND recipe_local_id = $2 RETURNING is_favorite",
    )
    .bind(username)
    .bind(recipe_local_id)
    .fetch_optional(pool)
    .await?;
    Ok(fav.unwrap_or(true))
}
