use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::social::models::{SocialComment, SocialPost},
};

pub async fn insert_post(
    pool: &PgPool,
    user_id: Option<Uuid>,
    author_name: &str,
    author_avatar: Option<&str>,
    author_role: &str,
    content: &str,
    recipe_id: Option<i32>,
    recipe_snapshot: Option<serde_json::Value>,
    tags_json: serde_json::Value,
) -> AppResult<SocialPost> {
    let post = sqlx::query_as::<_, SocialPost>(
        r#"INSERT INTO social_posts (
            user_id, author_name, author_avatar, author_role, content,
            recipe_id, recipe_snapshot, tags
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *"#,
    )
    .bind(user_id)
    .bind(author_name)
    .bind(author_avatar)
    .bind(author_role)
    .bind(content)
    .bind(recipe_id)
    .bind(recipe_snapshot)
    .bind(tags_json)
    .fetch_one(pool)
    .await?;
    Ok(post)
}

pub async fn toggle_post_like(pool: &PgPool, post_id: Uuid, username: &str) -> AppResult<(bool, i32)> {
    let already_liked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM social_post_likes WHERE post_id = $1 AND username = $2)",
    )
    .bind(post_id)
    .bind(username)
    .fetch_one(pool)
    .await?;

    if already_liked {
        sqlx::query("DELETE FROM social_post_likes WHERE post_id = $1 AND username = $2")
            .bind(post_id)
            .bind(username)
            .execute(pool)
            .await?;

        let count: i32 = sqlx::query_scalar(
            "UPDATE social_posts SET likes_count = GREATEST(likes_count - 1, 0) WHERE id = $1 RETURNING likes_count",
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(0);
        Ok((false, count))
    } else {
        sqlx::query("INSERT INTO social_post_likes (post_id, username) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(post_id)
            .bind(username)
            .execute(pool)
            .await?;

        let count: i32 = sqlx::query_scalar(
            "UPDATE social_posts SET likes_count = likes_count + 1 WHERE id = $1 RETURNING likes_count",
        )
        .bind(post_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(1);
        Ok((true, count))
    }
}

pub async fn list_comments(pool: &PgPool, post_id: Uuid) -> AppResult<Vec<SocialComment>> {
    let rows = sqlx::query_as::<_, SocialComment>(
        "SELECT * FROM social_comments WHERE post_id = $1 ORDER BY created_at ASC",
    )
    .bind(post_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn insert_comment(
    pool: &PgPool,
    post_id: Uuid,
    parent_id: Option<Uuid>,
    user_id: Option<Uuid>,
    author_name: &str,
    author_avatar: Option<&str>,
    content: &str,
) -> AppResult<SocialComment> {
    let comment = sqlx::query_as::<_, SocialComment>(
        r#"INSERT INTO social_comments (post_id, parent_id, user_id, author_name, author_avatar, content)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING *"#,
    )
    .bind(post_id)
    .bind(parent_id)
    .bind(user_id)
    .bind(author_name)
    .bind(author_avatar)
    .bind(content)
    .fetch_one(pool)
    .await?;

    sqlx::query("UPDATE social_posts SET comments_count = comments_count + 1 WHERE id = $1")
        .bind(post_id)
        .execute(pool)
        .await?;

    Ok(comment)
}

pub async fn increment_recipe_copy(pool: &PgPool, post_id: Uuid) -> AppResult<Option<SocialPost>> {
    let post = sqlx::query_as::<_, SocialPost>(
        "UPDATE social_posts SET copies_count = copies_count + 1 WHERE id = $1 RETURNING *",
    )
    .bind(post_id)
    .fetch_optional(pool)
    .await?;
    Ok(post)
}

pub async fn toggle_follow(pool: &PgPool, follower: &str, following: &str) -> AppResult<bool> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM social_follows WHERE follower_username = $1 AND following_username = $2)",
    )
    .bind(follower)
    .bind(following)
    .fetch_one(pool)
    .await?;

    if exists {
        sqlx::query("DELETE FROM social_follows WHERE follower_username = $1 AND following_username = $2")
            .bind(follower)
            .bind(following)
            .execute(pool)
            .await?;
        Ok(false)
    } else {
        sqlx::query("INSERT INTO social_follows (follower_username, following_username) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(follower)
            .bind(following)
            .execute(pool)
            .await?;
        Ok(true)
    }
}

pub async fn get_follow_stats(
    pool: &PgPool,
    username: &str,
    viewer: Option<&str>,
) -> AppResult<(i64, i64, bool)> {
    let followers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM social_follows WHERE following_username = $1")
        .bind(username)
        .fetch_one(pool)
        .await?;
    let following: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM social_follows WHERE follower_username = $1")
        .bind(username)
        .fetch_one(pool)
        .await?;
    let is_following = if let Some(v) = viewer {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM social_follows WHERE follower_username = $1 AND following_username = $2)")
            .bind(v)
            .bind(username)
            .fetch_one(pool)
            .await?
    } else {
        false
    };
    Ok((followers, following, is_following))
}
