use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_feed::error::SocialFeedError;

pub struct InsertThreadParams<'a> {
    pub id: Uuid,
    pub author_id: Uuid,
    pub kind: &'a str,
    pub parent_thread_id: Option<Uuid>,
    pub root_thread_id: Option<Uuid>,
    pub quoted_thread_id: Option<Uuid>,
    pub depth_level: i16,
    pub body_text: &'a str,
    pub has_media: bool,
    pub media_urls_json: serde_json::Value,
    pub attachment_type: Option<&'a str>,
    pub attachment_payload: Option<serde_json::Value>,
    pub is_pinned_on_profile: bool,
    pub recent_brew_snapshot: Option<serde_json::Value>,
}

pub async fn insert_thread_and_update_projection(
    pool: &PgPool,
    params: InsertThreadParams<'_>,
) -> Result<(), SocialFeedError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"INSERT INTO social_threads (
            id, author_id, kind, parent_thread_id, root_thread_id, quoted_thread_id,
            depth_level, body_text, has_media, media_urls, attachment_type, attachment_payload,
            is_pinned_on_profile
        ) VALUES ($1, $2, $3::thread_kind, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"#,
    )
    .bind(params.id)
    .bind(params.author_id)
    .bind(params.kind)
    .bind(params.parent_thread_id)
    .bind(params.root_thread_id)
    .bind(params.quoted_thread_id)
    .bind(params.depth_level)
    .bind(params.body_text)
    .bind(params.has_media)
    .bind(params.media_urls_json)
    .bind(params.attachment_type)
    .bind(params.attachment_payload)
    .bind(params.is_pinned_on_profile)
    .execute(&mut *tx)
    .await?;

    if let Some(parent_id) = params.parent_thread_id {
        sqlx::query("UPDATE social_threads SET replies_count = replies_count + 1 WHERE id = $1")
            .bind(parent_id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(quoted_id) = params.quoted_thread_id {
        sqlx::query("UPDATE social_threads SET reposts_count = reposts_count + 1 WHERE id = $1")
            .bind(quoted_id)
            .execute(&mut *tx)
            .await?;
    }

    let brew_inc: i32 = if params.kind == "BREW_LOG" { 1 } else { 0 };
    let recipe_inc: i32 = if params.kind == "CREATED_RECIPE" { 1 } else { 0 };
    let thread_inc: i32 = if params.kind != "REPLY" { 1 } else { 0 };

    sqlx::query(
        r#"UPDATE user_profile_projections
           SET total_threads_count = total_threads_count + $2,
               total_brews_count = total_brews_count + $3,
               total_recipes_created_count = total_recipes_created_count + $4,
               recent_brew_snapshot = COALESCE($5, recent_brew_snapshot),
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(params.author_id)
    .bind(thread_inc)
    .bind(brew_inc)
    .bind(recipe_inc)
    .bind(params.recent_brew_snapshot)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn soft_delete_thread(
    pool: &PgPool,
    thread_id: Uuid,
    actor_id: Uuid,
) -> Result<(), SocialFeedError> {
    let owner: Option<Uuid> =
        sqlx::query_scalar("SELECT author_id FROM social_threads WHERE id = $1 AND is_deleted = FALSE")
            .bind(thread_id)
            .fetch_optional(pool)
            .await?;

    let Some(author_id) = owner else {
        return Err(SocialFeedError::ThreadNotFound);
    };
    if author_id != actor_id {
        return Err(SocialFeedError::Forbidden);
    }

    sqlx::query("UPDATE social_threads SET is_deleted = TRUE WHERE id = $1")
        .bind(thread_id)
        .execute(pool)
        .await?;
    Ok(())
}
