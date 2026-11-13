use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::modules::communities::{
    dto::{
        CommunityLeaderboardEntryDto, CommunitySummaryDto, DiscoverCommunitiesQuery,
        DiscoverCommunitiesResponseDto,
    },
    error::CommunitiesError,
    models::{CommunityDiscoverRow, CommunityRow},
    repositories::community_query_repo,
    services::club_cache_service::ClubCacheService,
};

pub fn summary_from_row(row: &CommunityRow, viewer_role: Option<String>) -> CommunitySummaryDto {
    let tags = serde_json::from_value::<Vec<String>>(row.focus_tags.clone()).unwrap_or_default();
    CommunitySummaryDto {
        id: row.id,
        handle_slug: row.handle_slug.clone(),
        name: row.name.clone(),
        description: row.description.clone(),
        avatar_url: row.avatar_url.clone(),
        banner_url: row.banner_url.clone(),
        privacy_level: row.privacy_level.clone(),
        owner_id: row.owner_id,
        focus_tags: tags,
        members_count: row.members_count,
        threads_count: row.threads_count,
        shared_recipes_count: row.shared_recipes_count,
        total_club_xp: row.total_club_xp,
        is_member: viewer_role.is_some(),
        viewer_role,
        match_score_pct: None,
        created_at: row.created_at,
    }
}

pub fn summary_from_discover(row: CommunityDiscoverRow) -> CommunitySummaryDto {
    let tags = serde_json::from_value::<Vec<String>>(row.focus_tags).unwrap_or_default();
    let is_member = row.viewer_role.is_some();
    CommunitySummaryDto {
        id: row.id,
        handle_slug: row.handle_slug,
        name: row.name,
        description: row.description,
        avatar_url: row.avatar_url,
        banner_url: row.banner_url,
        privacy_level: row.privacy_level,
        owner_id: row.owner_id,
        focus_tags: tags,
        members_count: row.members_count,
        threads_count: row.threads_count,
        shared_recipes_count: row.shared_recipes_count,
        total_club_xp: row.total_club_xp,
        viewer_role: row.viewer_role,
        is_member,
        match_score_pct: None,
        created_at: row.created_at,
    }
}

pub async fn discover_communities(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    query: DiscoverCommunitiesQuery,
) -> Result<DiscoverCommunitiesResponseDto, CommunitiesError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 50);
    let offset = query.offset.unwrap_or(0).max(0);
    let my_clubs = match viewer_id {
        Some(uid) => community_query_repo::fetch_my_clubs(pg, uid)
            .await?
            .into_iter()
            .map(summary_from_discover)
            .collect(),
        None => Vec::new(),
    };

    let mut rows = community_query_repo::discover_non_secret_clubs(
        pg,
        viewer_id,
        query.q.as_deref(),
        limit + 1,
        offset,
    )
    .await?;
    let has_more = rows.len() > limit as usize;
    if has_more {
        rows.truncate(limit as usize);
    }

    Ok(DiscoverCommunitiesResponseDto {
        my_clubs,
        discover_clubs: rows.into_iter().map(summary_from_discover).collect(),
        has_more,
    })
}

pub async fn get_leaderboard(
    pg: &PgPool,
    cache: &ClubCacheService,
    viewer_id: Option<Uuid>,
    community_id: Uuid,
) -> Result<Vec<CommunityLeaderboardEntryDto>, CommunitiesError> {
    let club = cache.get_club(pg, community_id).await?;
    if club.privacy_level != "PUBLIC" {
        let Some(vid) = viewer_id else {
            return Err(CommunitiesError::Forbidden);
        };
        if !cache.is_member(pg, community_id, vid).await? {
            return Err(CommunitiesError::Forbidden);
        }
    }

    let rows = community_query_repo::fetch_leaderboard(pg, community_id, 50).await?;
    let target_ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
    let followed_set: HashSet<Uuid> = match viewer_id {
        Some(vid) if !target_ids.is_empty() => sqlx::query_scalar::<_, Uuid>(
            "SELECT following_id FROM user_follows WHERE follower_id = $1 AND status = 'ACCEPTED' AND following_id = ANY($2)",
        )
        .bind(vid)
        .bind(&target_ids)
        .fetch_all(pg)
        .await?
        .into_iter()
        .collect(),
        _ => HashSet::new(),
    };

    Ok(rows
        .into_iter()
        .map(|r| CommunityLeaderboardEntryDto {
            is_followed_by_viewer: followed_set.contains(&r.user_id),
            user_id: r.user_id,
            username: r.username,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            barista_level: r.barista_level,
            barista_rank_title: r.barista_rank_title,
            role: r.role,
            club_xp_contributed: r.club_xp_contributed,
            brews_shared_count: r.brews_shared_count,
            joined_at: r.joined_at,
        })
        .collect())
}
