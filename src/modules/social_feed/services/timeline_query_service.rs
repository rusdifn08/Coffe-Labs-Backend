use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::modules::social_feed::{
    cursor::TimelineCursor,
    dto::{
        CursorPaginatedFeedDto, ProfileFeedTab, SocialThreadItemDto, ThreadDetailResponseDto,
        ThreadReplyNodeDto,
    },
    error::SocialFeedError,
    repositories::{profile_projection_repo, thread_repo, timeline_repo},
    services::thread_hydrator,
};

pub async fn get_profile_tab_feed(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    target: &str,
    tab: ProfileFeedTab,
    raw_cursor: Option<&str>,
    limit: i64,
) -> Result<CursorPaginatedFeedDto<SocialThreadItemDto>, SocialFeedError> {
    let target_id = profile_projection_repo::resolve_target_user_id(pg, target, viewer_id).await?;
    profile_projection_repo::ensure_projection_exists(pg, target_id).await?;

    let page_size = limit.clamp(1, 50);
    let cursor = raw_cursor.map(TimelineCursor::decode_base64_url).transpose()?;

    let mut rows =
        thread_repo::fetch_profile_tab_rows(pg, target_id, tab, cursor, page_size + 1).await?;

    let has_more = rows.len() as i64 > page_size;
    if has_more {
        rows.truncate(page_size as usize);
    }

    let next_cursor = if has_more {
        rows.last()
            .map(|r| TimelineCursor::new(r.id, !r.is_pinned_on_profile).encode_base64_url())
            .transpose()?
    } else {
        None
    };

    let items = thread_hydrator::hydrate_threads_batch(pg, viewer_id, rows).await?;
    Ok(CursorPaginatedFeedDto {
        items,
        next_cursor,
        has_more,
    })
}

pub async fn get_global_timeline(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    scope: Option<&str>,
    raw_cursor: Option<&str>,
    limit: i64,
) -> Result<CursorPaginatedFeedDto<SocialThreadItemDto>, SocialFeedError> {
    let following_only = scope
        .unwrap_or("for_you")
        .trim()
        .eq_ignore_ascii_case("following");
    let page_size = limit.clamp(1, 50);
    let cursor_id = raw_cursor
        .map(TimelineCursor::decode_base64_url)
        .transpose()?
        .map(|c| c.last_id);

    let mut rows =
        timeline_repo::fetch_timeline_rows(pg, viewer_id, following_only, cursor_id, page_size + 1)
            .await?;

    let has_more = rows.len() as i64 > page_size;
    if has_more {
        rows.truncate(page_size as usize);
    }

    let next_cursor = if has_more {
        rows.last()
            .map(|r| TimelineCursor::new(r.id, true).encode_base64_url())
            .transpose()?
    } else {
        None
    };

    let items = thread_hydrator::hydrate_threads_batch(pg, viewer_id, rows).await?;
    Ok(CursorPaginatedFeedDto {
        items,
        next_cursor,
        has_more,
    })
}

pub async fn get_thread_with_nested_replies(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    thread_id: Uuid,
) -> Result<ThreadDetailResponseDto, SocialFeedError> {
    let target_row = thread_repo::fetch_thread_by_id(pg, thread_id).await?;
    let root_id = target_row.root_thread_id.unwrap_or(target_row.id);

    let root_row = if root_id == target_row.id {
        target_row
    } else {
        thread_repo::fetch_thread_by_id(pg, root_id).await?
    };

    let reply_rows = thread_repo::fetch_replies_by_root(pg, root_id).await?;
    let mut combined = Vec::with_capacity(1 + reply_rows.len());
    combined.push(root_row);
    combined.extend(reply_rows);

    let mut hydrated = thread_hydrator::hydrate_threads_batch(pg, viewer_id, combined).await?;
    let root = hydrated.remove(0);

    let mut children_map: HashMap<Uuid, Vec<SocialThreadItemDto>> = HashMap::new();
    for item in hydrated {
        let parent_key = item.parent_thread_id.unwrap_or(root.id);
        children_map.entry(parent_key).or_default().push(item);
    }

    let replies = build_reply_tree(root.id, &mut children_map);
    Ok(ThreadDetailResponseDto { root, replies })
}

fn build_reply_tree(
    parent_id: Uuid,
    children_map: &mut HashMap<Uuid, Vec<SocialThreadItemDto>>,
) -> Vec<ThreadReplyNodeDto> {
    let Some(direct_children) = children_map.remove(&parent_id) else {
        return Vec::new();
    };
    direct_children
        .into_iter()
        .map(|thread| {
            let tid = thread.id;
            let children = build_reply_tree(tid, children_map);
            ThreadReplyNodeDto { thread, children }
        })
        .collect()
}
