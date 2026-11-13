use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_feed::{
    cursor::TimelineCursor,
    dto::{ProfileFeedTab, QuotedThreadPreviewDto},
    error::SocialFeedError,
    models::SocialThreadWithAuthorRow,
    repositories::SELECT_THREAD_WITH_AUTHOR,
};

pub async fn fetch_profile_tab_rows(
    pool: &PgPool,
    author_id: Uuid,
    tab: ProfileFeedTab,
    cursor: Option<TimelineCursor>,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, SocialFeedError> {
    match tab {
        ProfileFeedTab::AllThreads => fetch_all_tab(pool, author_id, cursor, limit).await,
        ProfileFeedTab::BrewLogs => fetch_kind_tab(pool, author_id, "BREW_LOG", cursor, limit).await,
        ProfileFeedTab::CreatedRecipes => {
            fetch_kind_tab(pool, author_id, "CREATED_RECIPE", cursor, limit).await
        }
        ProfileFeedTab::Media => fetch_media_tab(pool, author_id, cursor, limit).await,
    }
}

async fn fetch_all_tab(
    pool: &PgPool,
    author_id: Uuid,
    cursor: Option<TimelineCursor>,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, SocialFeedError> {
    let base = SELECT_THREAD_WITH_AUTHOR;
    match cursor {
        Some(c) if c.pinned_passed => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.is_deleted = FALSE AND t.depth_level = 0 AND t.is_pinned_on_profile = FALSE AND t.id < $2 ORDER BY t.id DESC LIMIT $3");
            Ok(sqlx::query_as(&q).bind(author_id).bind(c.last_id).bind(limit).fetch_all(pool).await?)
        }
        Some(c) => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.is_deleted = FALSE AND t.depth_level = 0 AND ((t.is_pinned_on_profile = TRUE AND t.id < $2) OR t.is_pinned_on_profile = FALSE) ORDER BY t.is_pinned_on_profile DESC, t.id DESC LIMIT $3");
            Ok(sqlx::query_as(&q).bind(author_id).bind(c.last_id).bind(limit).fetch_all(pool).await?)
        }
        None => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.is_deleted = FALSE AND t.depth_level = 0 ORDER BY t.is_pinned_on_profile DESC, t.id DESC LIMIT $2");
            Ok(sqlx::query_as(&q).bind(author_id).bind(limit).fetch_all(pool).await?)
        }
    }
}

async fn fetch_kind_tab(
    pool: &PgPool,
    author_id: Uuid,
    kind: &str,
    cursor: Option<TimelineCursor>,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, SocialFeedError> {
    let base = SELECT_THREAD_WITH_AUTHOR;
    match cursor {
        Some(c) => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.kind = $2::thread_kind AND t.is_deleted = FALSE AND t.id < $3 ORDER BY t.id DESC LIMIT $4");
            Ok(sqlx::query_as(&q).bind(author_id).bind(kind).bind(c.last_id).bind(limit).fetch_all(pool).await?)
        }
        None => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.kind = $2::thread_kind AND t.is_deleted = FALSE ORDER BY t.id DESC LIMIT $3");
            Ok(sqlx::query_as(&q).bind(author_id).bind(kind).bind(limit).fetch_all(pool).await?)
        }
    }
}

async fn fetch_media_tab(
    pool: &PgPool,
    author_id: Uuid,
    cursor: Option<TimelineCursor>,
    limit: i64,
) -> Result<Vec<SocialThreadWithAuthorRow>, SocialFeedError> {
    let base = SELECT_THREAD_WITH_AUTHOR;
    match cursor {
        Some(c) => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.has_media = TRUE AND t.is_deleted = FALSE AND t.id < $2 ORDER BY t.id DESC LIMIT $3");
            Ok(sqlx::query_as(&q).bind(author_id).bind(c.last_id).bind(limit).fetch_all(pool).await?)
        }
        None => {
            let q = format!("{base} WHERE t.author_id = $1 AND t.has_media = TRUE AND t.is_deleted = FALSE ORDER BY t.id DESC LIMIT $2");
            Ok(sqlx::query_as(&q).bind(author_id).bind(limit).fetch_all(pool).await?)
        }
    }
}

pub async fn fetch_thread_by_id(
    pool: &PgPool,
    thread_id: Uuid,
) -> Result<SocialThreadWithAuthorRow, SocialFeedError> {
    let q = format!("{SELECT_THREAD_WITH_AUTHOR} WHERE t.id = $1 AND t.is_deleted = FALSE");
    sqlx::query_as::<_, SocialThreadWithAuthorRow>(&q)
        .bind(thread_id)
        .fetch_optional(pool)
        .await?
        .ok_or(SocialFeedError::ThreadNotFound)
}

pub async fn fetch_replies_by_root(
    pool: &PgPool,
    root_thread_id: Uuid,
) -> Result<Vec<SocialThreadWithAuthorRow>, SocialFeedError> {
    let q = format!("{SELECT_THREAD_WITH_AUTHOR} WHERE t.root_thread_id = $1 AND t.is_deleted = FALSE ORDER BY t.id ASC LIMIT 200");
    Ok(sqlx::query_as(&q).bind(root_thread_id).fetch_all(pool).await?)
}

pub async fn fetch_quoted_previews(
    pool: &PgPool,
    quoted_ids: &[Uuid],
) -> Result<Vec<QuotedThreadPreviewDto>, SocialFeedError> {
    if quoted_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, QuotedRow>(
        r#"SELECT t.id, t.author_id, p.username, p.display_name, p.avatar_url,
                  t.body_text, t.kind::text AS kind, t.created_at
           FROM social_threads t
           INNER JOIN user_profile_projections p ON p.user_id = t.author_id
           WHERE t.id = ANY($1::uuid[]) AND t.is_deleted = FALSE"#,
    )
    .bind(quoted_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| QuotedThreadPreviewDto {
            id: r.id,
            author_id: r.author_id,
            author_username: r.username,
            author_display_name: r.display_name,
            author_avatar_url: r.avatar_url,
            body_text: r.body_text,
            kind: r.kind,
            created_at: r.created_at,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct QuotedRow {
    id: Uuid,
    author_id: Uuid,
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    body_text: String,
    kind: String,
    created_at: chrono::DateTime<chrono::Utc>,
}
