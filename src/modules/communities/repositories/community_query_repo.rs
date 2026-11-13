use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::communities::{
    error::CommunitiesError,
    models::{
        CommunityChannelRow, CommunityDiscoverRow, CommunityLeaderboardRow, CommunityRow,
    },
};

const SELECT_COMMUNITY_FIELDS: &str = r#"
SELECT id, handle_slug, name, description, avatar_url, banner_url,
       privacy_level::text AS privacy_level, owner_id, focus_tags,
       taste_feature_vector, members_count, threads_count,
       shared_recipes_count, total_club_xp, created_at, updated_at
FROM coffee_communities
"#;

pub async fn fetch_community_by_id(
    pg: &PgPool,
    community_id: Uuid,
) -> Result<Option<CommunityRow>, CommunitiesError> {
    let row = sqlx::query_as::<_, CommunityRow>(&format!(
        "{SELECT_COMMUNITY_FIELDS} WHERE id = $1"
    ))
    .bind(community_id)
    .fetch_optional(pg)
    .await?;
    Ok(row)
}

pub async fn fetch_viewer_role(
    pg: &PgPool,
    community_id: Uuid,
    user_id: Uuid,
) -> Result<Option<String>, CommunitiesError> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role::text FROM community_members WHERE community_id = $1 AND user_id = $2",
    )
    .bind(community_id)
    .bind(user_id)
    .fetch_optional(pg)
    .await?;
    Ok(role)
}

pub async fn fetch_club_member_ids(
    pg: &PgPool,
    community_id: Uuid,
) -> Result<Vec<Uuid>, CommunitiesError> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM community_members WHERE community_id = $1",
    )
    .bind(community_id)
    .fetch_all(pg)
    .await?;
    Ok(ids)
}

pub async fn fetch_channels(
    pg: &PgPool,
    community_id: Uuid,
) -> Result<Vec<CommunityChannelRow>, CommunitiesError> {
    let rows = sqlx::query_as::<_, CommunityChannelRow>(
        r#"SELECT id, community_id, name, description, kind::text AS kind,
                  linked_conversation_id, display_order, is_readonly_announcement, created_at
           FROM community_channels
           WHERE community_id = $1
           ORDER BY display_order ASC, created_at ASC"#,
    )
    .bind(community_id)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

pub async fn fetch_my_clubs(
    pg: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CommunityDiscoverRow>, CommunitiesError> {
    let rows = sqlx::query_as::<_, CommunityDiscoverRow>(
        r#"SELECT c.id, c.handle_slug, c.name, c.description, c.avatar_url, c.banner_url,
                  c.privacy_level::text AS privacy_level, c.owner_id, c.focus_tags,
                  c.taste_feature_vector, c.members_count, c.threads_count,
                  c.shared_recipes_count, c.total_club_xp,
                  cm.role::text AS viewer_role, c.created_at
           FROM coffee_communities c
           INNER JOIN community_members cm ON cm.community_id = c.id AND cm.user_id = $1
           ORDER BY cm.joined_at DESC
           LIMIT 30"#,
    )
    .bind(user_id)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

pub async fn discover_non_secret_clubs(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    search: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<CommunityDiscoverRow>, CommunitiesError> {
    let pattern = search
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s));

    let rows = sqlx::query_as::<_, CommunityDiscoverRow>(
        r#"SELECT c.id, c.handle_slug, c.name, c.description, c.avatar_url, c.banner_url,
                  c.privacy_level::text AS privacy_level, c.owner_id, c.focus_tags,
                  c.taste_feature_vector, c.members_count, c.threads_count,
                  c.shared_recipes_count, c.total_club_xp,
                  cm.role::text AS viewer_role, c.created_at
           FROM coffee_communities c
           LEFT JOIN community_members cm
             ON cm.community_id = c.id AND cm.user_id = $1
           WHERE c.privacy_level <> 'SECRET_VAULT'
             AND ($2::text IS NULL OR c.name ILIKE $2 OR c.handle_slug ILIKE $2 OR c.description ILIKE $2)
           ORDER BY c.members_count DESC, c.total_club_xp DESC, c.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(viewer_id)
    .bind(pattern)
    .bind(limit)
    .bind(offset)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

pub async fn fetch_leaderboard(
    pg: &PgPool,
    community_id: Uuid,
    limit: i64,
) -> Result<Vec<CommunityLeaderboardRow>, CommunitiesError> {
    let rows = sqlx::query_as::<_, CommunityLeaderboardRow>(
        r#"SELECT cm.user_id,
                  COALESCE(p.username, u.username, 'barista') AS username,
                  COALESCE(p.display_name, NULLIF(u.full_name, ''), u.username, 'Barista') AS display_name,
                  p.avatar_url,
                  COALESCE(p.barista_level, 1) AS barista_level,
                  COALESCE(p.barista_rank_title, 'Home Brewer') AS barista_rank_title,
                  cm.role::text AS role,
                  cm.club_xp_contributed,
                  cm.brews_shared_count,
                  cm.joined_at
           FROM community_members cm
           INNER JOIN users u ON u.id = cm.user_id
           LEFT JOIN user_profile_projections p ON p.user_id = cm.user_id
           WHERE cm.community_id = $1
           ORDER BY cm.club_xp_contributed DESC, cm.joined_at ASC
           LIMIT $2"#,
    )
    .bind(community_id)
    .bind(limit)
    .fetch_all(pg)
    .await?;
    Ok(rows)
}
