use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_feed::{
    error::SocialFeedError,
    models::SocialThreadWithAuthorRow,
    repositories::SELECT_THREAD_WITH_AUTHOR,
};

pub async fn fetch_timeline_rows(
    pool: &PgPool,
    viewer_id: Option<Uuid>,
    following_only: bool,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, SocialFeedError> {
    if following_only {
        let Some(uid) = viewer_id else {
            return Ok(Vec::new());
        };
        let q = if cursor.is_some() {
            format!(
                "{SELECT_THREAD_WITH_AUTHOR}
                 INNER JOIN user_follows f ON f.following_id = t.author_id AND f.follower_id = $1 AND f.status = 'ACCEPTED'
                 WHERE t.is_deleted = FALSE AND t.depth_level = 0
                   AND (t.community_id IS NULL OR NOT EXISTS (SELECT 1 FROM coffee_communities c WHERE c.id = t.community_id AND c.privacy_level = 'SECRET_VAULT'))
                   AND t.id < $2
                 ORDER BY t.id DESC LIMIT $3"
            )
        } else {
            format!(
                "{SELECT_THREAD_WITH_AUTHOR}
                 INNER JOIN user_follows f ON f.following_id = t.author_id AND f.follower_id = $1 AND f.status = 'ACCEPTED'
                 WHERE t.is_deleted = FALSE AND t.depth_level = 0
                   AND (t.community_id IS NULL OR NOT EXISTS (SELECT 1 FROM coffee_communities c WHERE c.id = t.community_id AND c.privacy_level = 'SECRET_VAULT'))
                 ORDER BY t.id DESC LIMIT $2"
            )
        };
        return match cursor {
            Some(cid) => Ok(sqlx::query_as(&q)
                .bind(uid)
                .bind(cid)
                .bind(limit)
                .fetch_all(pool)
                .await?),
            None => Ok(sqlx::query_as(&q)
                .bind(uid)
                .bind(limit)
                .fetch_all(pool)
                .await?),
        };
    }

    let q = if cursor.is_some() {
        format!(
            "{SELECT_THREAD_WITH_AUTHOR}
             WHERE t.is_deleted = FALSE AND t.depth_level = 0
               AND (t.community_id IS NULL OR NOT EXISTS (SELECT 1 FROM coffee_communities c WHERE c.id = t.community_id AND c.privacy_level = 'SECRET_VAULT'))
               AND t.id < $1
             ORDER BY t.id DESC LIMIT $2"
        )
    } else {
        format!(
            "{SELECT_THREAD_WITH_AUTHOR}
             WHERE t.is_deleted = FALSE AND t.depth_level = 0
               AND (t.community_id IS NULL OR NOT EXISTS (SELECT 1 FROM coffee_communities c WHERE c.id = t.community_id AND c.privacy_level = 'SECRET_VAULT'))
             ORDER BY t.id DESC LIMIT $1"
        )
    };
    match cursor {
        Some(cid) => Ok(sqlx::query_as(&q)
            .bind(cid)
            .bind(limit)
            .fetch_all(pool)
            .await?),
        None => Ok(sqlx::query_as(&q).bind(limit).fetch_all(pool).await?),
    }
}
