use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::{core::error::AppResult, modules::social::models::SocialPost};

pub async fn fetch_feed_posts(
    pool: &PgPool,
    sort: &str,
    limit: i64,
    offset: i64,
    cursor: Option<(DateTime<Utc>, Uuid)>,
) -> AppResult<Vec<SocialPost>> {
    if let Some((cursor_ts, cursor_id)) = cursor {
        let rows = sqlx::query_as::<_, SocialPost>(
            r#"SELECT * FROM social_posts
               WHERE (created_at, id) < ($1, $2)
               ORDER BY created_at DESC, id DESC
               LIMIT $3"#,
        )
        .bind(cursor_ts)
        .bind(cursor_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
        return Ok(rows);
    }

    let rows = match sort {
        "popular" => {
            sqlx::query_as::<_, SocialPost>(
                r#"SELECT * FROM social_posts
                   ORDER BY (likes_count * 2 + comments_count * 3 + copies_count * 4) DESC, created_at DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        "recipes" => {
            sqlx::query_as::<_, SocialPost>(
                r#"SELECT * FROM social_posts
                   WHERE recipe_snapshot IS NOT NULL
                   ORDER BY created_at DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        _ => {
            sqlx::query_as::<_, SocialPost>(
                r#"SELECT * FROM social_posts
                   ORDER BY created_at DESC, id DESC
                   LIMIT $1 OFFSET $2"#,
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

pub async fn fetch_user_liked_post_ids(
    pool: &PgPool,
    username: &str,
    post_ids: &[Uuid],
) -> AppResult<HashSet<Uuid>> {
    if username.is_empty() || post_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let rows = sqlx::query_scalar::<_, Uuid>(
        "SELECT post_id FROM social_post_likes WHERE username = $1 AND post_id = ANY($2)",
    )
    .bind(username)
    .bind(post_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}
