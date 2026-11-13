use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::modules::social_feed::error::SocialFeedError;

pub async fn fetch_user_liked_ids(
    pool: &PgPool,
    user_id: Uuid,
    thread_ids: &[Uuid],
) -> Result<HashSet<Uuid>, SocialFeedError> {
    if thread_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT thread_id FROM social_thread_likes WHERE user_id = $1 AND thread_id = ANY($2::uuid[])",
    )
    .bind(user_id)
    .bind(thread_ids)
    .fetch_all(pool)
    .await?;
    Ok(ids.into_iter().collect())
}

pub async fn fetch_user_reposted_ids(
    pool: &PgPool,
    user_id: Uuid,
    thread_ids: &[Uuid],
) -> Result<HashSet<Uuid>, SocialFeedError> {
    if thread_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT thread_id FROM social_thread_reposts WHERE user_id = $1 AND thread_id = ANY($2::uuid[])",
    )
    .bind(user_id)
    .bind(thread_ids)
    .fetch_all(pool)
    .await?;
    Ok(ids.into_iter().collect())
}

pub async fn toggle_like_atomic(
    pool: &PgPool,
    thread_id: Uuid,
    user_id: Uuid,
) -> Result<(bool, i32), SocialFeedError> {
    let mut tx = pool.begin().await?;
    let del = sqlx::query(
        "DELETE FROM social_thread_likes WHERE thread_id = $1 AND user_id = $2",
    )
    .bind(thread_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let is_liked = if del.rows_affected() > 0 {
        sqlx::query("UPDATE social_threads SET likes_count = GREATEST(likes_count - 1, 0) WHERE id = $1")
            .bind(thread_id)
            .execute(&mut *tx)
            .await?;
        false
    } else {
        sqlx::query(
            "INSERT INTO social_thread_likes (thread_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE social_threads SET likes_count = likes_count + 1 WHERE id = $1")
            .bind(thread_id)
            .execute(&mut *tx)
            .await?;
        true
    };

    let likes_count: i32 = sqlx::query_scalar("SELECT likes_count FROM social_threads WHERE id = $1")
        .bind(thread_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SocialFeedError::ThreadNotFound)?;

    tx.commit().await?;
    Ok((is_liked, likes_count))
}

pub async fn toggle_repost_atomic(
    pool: &PgPool,
    thread_id: Uuid,
    user_id: Uuid,
) -> Result<(bool, i32), SocialFeedError> {
    let mut tx = pool.begin().await?;
    let del = sqlx::query(
        "DELETE FROM social_thread_reposts WHERE thread_id = $1 AND user_id = $2",
    )
    .bind(thread_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let is_reposted = if del.rows_affected() > 0 {
        sqlx::query("UPDATE social_threads SET reposts_count = GREATEST(reposts_count - 1, 0) WHERE id = $1")
            .bind(thread_id)
            .execute(&mut *tx)
            .await?;
        false
    } else {
        sqlx::query(
            "INSERT INTO social_thread_reposts (thread_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(thread_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE social_threads SET reposts_count = reposts_count + 1 WHERE id = $1")
            .bind(thread_id)
            .execute(&mut *tx)
            .await?;
        true
    };

    let reposts_count: i32 = sqlx::query_scalar("SELECT reposts_count FROM social_threads WHERE id = $1")
        .bind(thread_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SocialFeedError::ThreadNotFound)?;

    tx.commit().await?;
    Ok((is_reposted, reposts_count))
}

pub async fn increment_try_recipe(
    pool: &PgPool,
    thread_id: Uuid,
) -> Result<(Uuid, i32), SocialFeedError> {
    let row: Option<(Uuid, i32)> = sqlx::query_as(
        r#"UPDATE social_threads
           SET try_recipe_count = try_recipe_count + 1
           WHERE id = $1 AND is_deleted = FALSE
           RETURNING author_id, try_recipe_count"#,
    )
    .bind(thread_id)
    .fetch_optional(pool)
    .await?;

    let (author_id, new_count) = row.ok_or(SocialFeedError::ThreadNotFound)?;

    sqlx::query(
        r#"UPDATE user_profile_projections
           SET total_recipe_tries_received = total_recipe_tries_received + 1,
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(author_id)
    .execute(pool)
    .await?;

    Ok((author_id, new_count))
}
