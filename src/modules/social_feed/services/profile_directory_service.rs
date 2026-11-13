use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::modules::social_feed::{
    attachments::RecentBrewSnapshot,
    cursor::DirectoryCursor,
    dto::{CursorPaginatedFeedDto, ProfileDirectoryEntryDto},
    error::SocialFeedError,
    models::ProfileDirectoryRow,
    repositories::{follow_repo, profile_projection_repo},
};

fn map_directory_entry(
    row: ProfileDirectoryRow,
    followed_set: &HashSet<Uuid>,
) -> ProfileDirectoryEntryDto {
    let is_followed_by_viewer = followed_set.contains(&row.user_id);
    ProfileDirectoryEntryDto {
        user_id: row.user_id,
        username: row.username,
        display_name: row.display_name,
        bio: row.bio,
        avatar_url: row.avatar_url,
        account_tier: row.account_tier,
        total_xp: row.total_xp,
        barista_level: row.barista_level,
        barista_rank_title: row.barista_rank_title,
        followers_count: row.followers_count,
        total_threads_count: row.total_threads_count,
        recent_brew_snapshot: row
            .recent_brew_snapshot
            .and_then(|v| serde_json::from_value::<RecentBrewSnapshot>(v).ok()),
        is_followed_by_viewer,
    }
}

pub async fn explore_directory(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    search: Option<&str>,
    raw_cursor: Option<&str>,
    limit: i64,
) -> Result<CursorPaginatedFeedDto<ProfileDirectoryEntryDto>, SocialFeedError> {
    let page_size = limit.clamp(1, 50);
    let decoded = raw_cursor
        .map(DirectoryCursor::decode_base64_url)
        .transpose()?;
    let mut rows =
        follow_repo::list_directory_profiles(pg, search, decoded, page_size + 1).await?;

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

    let candidate_ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
    let followed_set = match viewer_id {
        Some(vid) => follow_repo::fetch_viewer_followed_ids(pg, vid, &candidate_ids).await?,
        None => HashSet::new(),
    };

    let items = rows
        .into_iter()
        .map(|r| map_directory_entry(r, &followed_set))
        .collect();

    Ok(CursorPaginatedFeedDto {
        items,
        next_cursor,
        has_more,
    })
}

pub async fn list_connections(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    target: &str,
    fetch_followers: bool,
    limit: i64,
) -> Result<Vec<ProfileDirectoryEntryDto>, SocialFeedError> {
    let target_id =
        profile_projection_repo::resolve_target_user_id(pg, target, viewer_id).await?;
    let rows = follow_repo::list_follow_connections(
        pg,
        target_id,
        fetch_followers,
        limit.clamp(1, 100),
    )
    .await?;

    let ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
    let followed_set = match viewer_id {
        Some(vid) => follow_repo::fetch_viewer_followed_ids(pg, vid, &ids).await?,
        None => HashSet::new(),
    };

    Ok(rows
        .into_iter()
        .map(|r| map_directory_entry(r, &followed_set))
        .collect())
}
