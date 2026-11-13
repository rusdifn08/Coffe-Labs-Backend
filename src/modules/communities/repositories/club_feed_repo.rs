use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::{
    communities::error::CommunitiesError,
    social_feed::{models::SocialThreadWithAuthorRow, repositories::SELECT_THREAD_WITH_AUTHOR},
};

pub const CLUB_POST_XP_REWARD: i64 = 25;

pub struct InsertClubThreadParams<'a> {
    pub id: Uuid,
    pub community_id: Uuid,
    pub author_id: Uuid,
    pub kind: &'a str,
    pub quoted_thread_id: Option<Uuid>,
    pub body_text: &'a str,
    pub has_media: bool,
    pub media_urls_json: serde_json::Value,
    pub attachment_type: Option<&'a str>,
    pub attachment_payload: Option<serde_json::Value>,
    pub is_pinned: bool,
    pub is_recipe_vault_item: bool,
    pub is_brew_activity: bool,
    pub taste_feature_vector: &'a [f32],
}

pub async fn insert_club_thread_atomic(
    pg: &PgPool,
    params: InsertClubThreadParams<'_>,
) -> Result<(), CommunitiesError> {
    let mut tx = pg.begin().await?;

    sqlx::query(
        r#"INSERT INTO social_threads (
            id, author_id, community_id, kind, quoted_thread_id, depth_level,
            body_text, has_media, media_urls, attachment_type, attachment_payload,
            is_pinned_on_profile, taste_feature_vector
        ) VALUES ($1, $2, $3, $4::thread_kind, $5, 0, $6, $7, $8, $9, $10, $11, $12)"#,
    )
    .bind(params.id)
    .bind(params.author_id)
    .bind(params.community_id)
    .bind(params.kind)
    .bind(params.quoted_thread_id)
    .bind(params.body_text)
    .bind(params.has_media)
    .bind(params.media_urls_json)
    .bind(params.attachment_type)
    .bind(params.attachment_payload)
    .bind(params.is_pinned)
    .bind(params.taste_feature_vector)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"UPDATE coffee_communities
           SET threads_count = threads_count + 1,
               shared_recipes_count = shared_recipes_count + CASE WHEN $2 THEN 1 ELSE 0 END,
               total_club_xp = total_club_xp + $3,
               updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(params.community_id)
    .bind(params.is_recipe_vault_item)
    .bind(CLUB_POST_XP_REWARD)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"UPDATE community_members
           SET club_xp_contributed = club_xp_contributed + $3,
               brews_shared_count = brews_shared_count + CASE WHEN $4 THEN 1 ELSE 0 END
           WHERE community_id = $1 AND user_id = $2"#,
    )
    .bind(params.community_id)
    .bind(params.author_id)
    .bind(CLUB_POST_XP_REWARD)
    .bind(params.is_brew_activity)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"UPDATE user_profile_projections
           SET total_threads_count = total_threads_count + 1,
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(params.author_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn fetch_club_feed_rows(
    pg: &PgPool,
    community_id: Uuid,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, CommunitiesError> {
    let q = if cursor.is_some() {
        format!(
            "{SELECT_THREAD_WITH_AUTHOR}
             WHERE t.community_id = $1 AND t.is_deleted = FALSE AND t.id < $2
             ORDER BY t.is_pinned_on_profile DESC, t.id DESC
             LIMIT $3"
        )
    } else {
        format!(
            "{SELECT_THREAD_WITH_AUTHOR}
             WHERE t.community_id = $1 AND t.is_deleted = FALSE
             ORDER BY t.is_pinned_on_profile DESC, t.id DESC
             LIMIT $2"
        )
    };

    let rows = match cursor {
        Some(cid) => sqlx::query_as(&q)
            .bind(community_id)
            .bind(cid)
            .bind(limit)
            .fetch_all(pg)
            .await?,
        None => sqlx::query_as(&q)
            .bind(community_id)
            .bind(limit)
            .fetch_all(pg)
            .await?,
    };
    Ok(rows)
}

pub async fn fetch_club_recipe_vault_rows(
    pg: &PgPool,
    community_id: Uuid,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, CommunitiesError> {
    let q = format!(
        "{SELECT_THREAD_WITH_AUTHOR}
         WHERE t.community_id = $1
           AND t.is_deleted = FALSE
           AND t.attachment_type = 'RECIPE_CARD'
         ORDER BY t.try_recipe_count DESC, t.id DESC
         LIMIT $2"
    );
    let rows = sqlx::query_as(&q)
        .bind(community_id)
        .bind(limit)
        .fetch_all(pg)
        .await?;
    Ok(rows)
}
