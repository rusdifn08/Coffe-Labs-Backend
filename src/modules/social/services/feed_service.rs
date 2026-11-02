use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::AppResult,
        pagination::{encode_cursor, CursorPage, CursorQuery},
    },
    modules::social::{
        dto::{SocialFeedQuery, SocialPostResponse},
        models::SocialPost,
        repositories::feed_repo,
    },
};

pub fn map_post_to_response(
    p: SocialPost,
    is_liked: bool,
    next_cursor: Option<String>,
) -> SocialPostResponse {
    let tags: Vec<String> = match p.tags {
        Some(serde_json::Value::Array(arr)) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    };

    SocialPostResponse {
        id: p.id,
        user_id: p.user_id,
        author_name: p.author_name,
        author_avatar: p.author_avatar,
        author_role: p.author_role,
        content: p.content,
        recipe_id: p.recipe_id,
        recipe_snapshot: p.recipe_snapshot,
        likes_count: p.likes_count,
        comments_count: p.comments_count,
        copies_count: p.copies_count,
        tags,
        is_liked_by_me: is_liked,
        next_cursor,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

pub async fn get_feed_list(
    pool: &PgPool,
    query: SocialFeedQuery,
) -> AppResult<Vec<SocialPostResponse>> {
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let sort = query.sort.as_deref().unwrap_or("latest");
    let username = query
        .username
        .as_deref()
        .map(|u| u.trim().to_lowercase())
        .unwrap_or_default();

    let cursor_q = CursorQuery {
        cursor: query.cursor.clone(),
        limit: Some(limit),
    };
    let clamped = cursor_q.clamped_limit();
    let decoded_cursor = cursor_q.decode_cursor();
    let tag_filter = query.tag.as_deref().map(|t| t.trim().to_lowercase());

    let posts = feed_repo::fetch_feed_posts(pool, sort, clamped, offset, decoded_cursor).await?;
    let post_ids: Vec<Uuid> = posts.iter().map(|p| p.id).collect();
    let liked_set = feed_repo::fetch_user_liked_post_ids(pool, &username, &post_ids).await?;

    let next_cursor = posts.last().map(|last| encode_cursor(last.created_at, last.id));

    let items = posts
        .into_iter()
        .map(|p| {
            let is_liked = liked_set.contains(&p.id);
            map_post_to_response(p, is_liked, next_cursor.clone())
        })
        .filter(|item| {
            if let Some(ref tf) = tag_filter {
                if !tf.is_empty() {
                    return item.tags.iter().any(|t| t.to_lowercase().contains(tf));
                }
            }
            true
        })
        .collect();

    Ok(items)
}

pub async fn get_cursor_feed(
    pool: &PgPool,
    query: SocialFeedQuery,
) -> AppResult<CursorPage<SocialPostResponse>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let mut fetch_query = query;
    fetch_query.limit = Some(limit + 1);

    let mut items = get_feed_list(pool, fetch_query).await?;
    let has_more = items.len() as i64 > limit;
    if has_more {
        items.truncate(limit as usize);
    }

    let next_cursor = if has_more {
        items.last().map(|last| encode_cursor(last.created_at, last.id))
    } else {
        None
    };

    Ok(CursorPage::new(items, next_cursor, has_more))
}
