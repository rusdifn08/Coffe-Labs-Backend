use fred::clients::RedisPool;
use sqlx::PgPool;
use std::{collections::HashSet, sync::Arc};
use uuid::Uuid;

use crate::{
    core::{
        cache::AppCaches,
        redis::{redis_get_bounded, redis_set_ex_bounded},
    },
    modules::social_feed::{
        cursor::DirectoryCursor,
        dto::{CursorPaginatedFeedDto, ProfileDirectoryEntryDto},
        error::SocialFeedError,
        repositories::{directory_repo, follow_repo},
    },
};

pub use super::directory_connections_service::{list_connections, map_directory_entry};

const REDIS_DIR_PREFIX: &str = "social:dir:v1:";
const REDIS_DIR_TTL_SECS: i64 = 30;

async fn load_base_directory_page(
    pg: &PgPool,
    caches: &AppCaches,
    redis_pool: Option<&RedisPool>,
    search: Option<&str>,
    raw_cursor: Option<&str>,
    page_size: i64,
) -> Result<CursorPaginatedFeedDto<ProfileDirectoryEntryDto>, SocialFeedError> {
    let normalized = directory_repo::normalize_search_query(search);
    let can_cache = raw_cursor.is_none();
    let cache_key = format!("{normalized}:{page_size}");

    if can_cache {
        if let Some(hit) = caches.directory_l1.get(&cache_key).await {
            if let Ok(parsed) = serde_json::from_value::<
                CursorPaginatedFeedDto<ProfileDirectoryEntryDto>,
            >(hit.as_ref().clone())
            {
                return Ok(parsed);
            }
        }
        if let Some(redis) = redis_pool {
            let rkey = format!("{REDIS_DIR_PREFIX}{cache_key}");
            if let Some(raw) = redis_get_bounded(redis, &rkey).await {
                if let Ok(parsed) = serde_json::from_str::<
                    CursorPaginatedFeedDto<ProfileDirectoryEntryDto>,
                >(&raw)
                {
                    if let Ok(val) = serde_json::to_value(&parsed) {
                        caches
                            .directory_l1
                            .insert(cache_key.clone(), Arc::new(val))
                            .await;
                    }
                    return Ok(parsed);
                }
            }
        }
    }

    let decoded = raw_cursor
        .map(DirectoryCursor::decode_base64_url)
        .transpose()?;
    let mut rows =
        directory_repo::search_directory_profiles(pg, search, decoded, page_size + 1).await?;

    let has_more = rows.len() as i64 > page_size;
    if has_more {
        rows.truncate(page_size as usize);
    }

    let next_cursor = if has_more {
        rows.last()
            .map(|r| {
                DirectoryCursor {
                    total_xp: r.total_xp,
                    user_id: r.user_id,
                }
                .encode_base64_url()
            })
            .transpose()?
    } else {
        None
    };

    let empty = HashSet::new();
    let dto = CursorPaginatedFeedDto {
        items: rows
            .into_iter()
            .map(|r| map_directory_entry(r, &empty))
            .collect(),
        next_cursor,
        has_more,
    };

    if can_cache && !dto.items.is_empty() {
        if let Ok(val) = serde_json::to_value(&dto) {
            caches
                .directory_l1
                .insert(cache_key.clone(), Arc::new(val))
                .await;
        }
        if let Some(redis) = redis_pool {
            if let Ok(serialized) = serde_json::to_string(&dto) {
                let rkey = format!("{REDIS_DIR_PREFIX}{cache_key}");
                redis_set_ex_bounded(redis, &rkey, serialized, REDIS_DIR_TTL_SECS).await;
            }
        }
    }

    Ok(dto)
}

async fn load_viewer_followed_set(
    pg: &PgPool,
    caches: &AppCaches,
    viewer_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<HashSet<Uuid>, SocialFeedError> {
    if candidate_ids.is_empty() {
        return Ok(HashSet::new());
    }
    if let Some(cached) = caches.viewer_followed_l1.get(&viewer_id).await {
        return Ok((*cached).clone());
    }
    let set = follow_repo::fetch_viewer_followed_ids(pg, viewer_id, candidate_ids).await?;
    caches
        .viewer_followed_l1
        .insert(viewer_id, Arc::new(set.clone()))
        .await;
    Ok(set)
}

pub async fn explore_directory_cached(
    pg: &PgPool,
    caches: &AppCaches,
    redis_pool: Option<&RedisPool>,
    viewer_id: Option<Uuid>,
    search: Option<&str>,
    raw_cursor: Option<&str>,
    limit: i64,
) -> Result<CursorPaginatedFeedDto<ProfileDirectoryEntryDto>, SocialFeedError> {
    let page_size = limit.clamp(1, 50);
    let mut page =
        load_base_directory_page(pg, caches, redis_pool, search, raw_cursor, page_size).await?;

    if let Some(vid) = viewer_id {
        let candidate_ids: Vec<Uuid> = page.items.iter().map(|i| i.user_id).collect();
        let followed_set = load_viewer_followed_set(pg, caches, vid, &candidate_ids).await?;
        for item in &mut page.items {
            item.is_followed_by_viewer = followed_set.contains(&item.user_id);
        }
    }

    Ok(page)
}
