use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_graph::{
    error::SocialGraphError,
    models::{FollowEdgeUserRow, GraphCursor},
};

pub async fn list_followers_keyset(
    pool: &PgPool,
    target_id: Uuid,
    cursor: Option<GraphCursor>,
    limit: i64,
) -> Result<Vec<FollowEdgeUserRow>, SocialGraphError> {
    list_by_edge_direction(pool, target_id, true, "ACCEPTED", cursor, limit).await
}

pub async fn list_following_keyset(
    pool: &PgPool,
    target_id: Uuid,
    cursor: Option<GraphCursor>,
    limit: i64,
) -> Result<Vec<FollowEdgeUserRow>, SocialGraphError> {
    list_by_edge_direction(pool, target_id, false, "ACCEPTED", cursor, limit).await
}

pub async fn list_pending_requests_keyset(
    pool: &PgPool,
    owner_id: Uuid,
    cursor: Option<GraphCursor>,
    limit: i64,
) -> Result<Vec<FollowEdgeUserRow>, SocialGraphError> {
    list_by_edge_direction(pool, owner_id, true, "PENDING", cursor, limit).await
}

async fn list_by_edge_direction(
    pool: &PgPool,
    owner_id: Uuid,
    incoming: bool,
    status: &str,
    cursor: Option<GraphCursor>,
    limit: i64,
) -> Result<Vec<FollowEdgeUserRow>, SocialGraphError> {
    let (filter_col, join_col) = if incoming {
        ("following_id", "follower_id")
    } else {
        ("follower_id", "following_id")
    };
    let cursor_clause = if cursor.is_some() {
        format!("AND (f.created_at, f.{join_col}) < ($3, $4)")
    } else {
        String::new()
    };
    let limit_param = if cursor.is_some() { "$5" } else { "$3" };
    let sql = format!(
        "SELECT u.id AS user_id,
                COALESCE(NULLIF(p.username, ''), u.username) AS username,
                COALESCE(NULLIF(p.display_name, ''), NULLIF(u.full_name, ''), u.username) AS display_name,
                COALESCE(p.avatar_url, u.avatar_url) AS avatar_url,
                COALESCE(p.barista_level, 1) AS barista_level,
                COALESCE(NULLIF(p.account_tier, ''), INITCAP(COALESCE(u.tier, 'trial'))) AS account_tier,
                COALESCE(p.is_private_account, FALSE) AS is_private_account,
                f.created_at AS edge_created_at
         FROM user_follows f
         INNER JOIN users u ON u.id = f.{join_col}
         LEFT JOIN user_profile_projections p ON p.user_id = f.{join_col}
         WHERE f.{filter_col} = $1 AND f.status::text = $2
         {cursor_clause}
         ORDER BY f.created_at DESC, f.{join_col} DESC
         LIMIT {limit_param}"
    );
    let mut query = sqlx::query_as::<_, FollowEdgeUserRow>(&sql)
        .bind(owner_id)
        .bind(status);
    if let Some(c) = cursor {
        query = query.bind(c.created_at).bind(c.user_id);
    }
    Ok(query.bind(limit).fetch_all(pool).await?)
}

pub async fn list_mutual_connections_keyset(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
    cursor: Option<GraphCursor>,
    limit: i64,
) -> Result<Vec<FollowEdgeUserRow>, SocialGraphError> {
    let cursor_clause = if cursor.is_some() {
        "AND (b.created_at, a.following_id) < ($3, $4)"
    } else {
        ""
    };
    let limit_param = if cursor.is_some() { "$5" } else { "$3" };
    let sql = format!(
        "SELECT u.id AS user_id,
                COALESCE(NULLIF(p.username, ''), u.username) AS username,
                COALESCE(NULLIF(p.display_name, ''), NULLIF(u.full_name, ''), u.username) AS display_name,
                COALESCE(p.avatar_url, u.avatar_url) AS avatar_url,
                COALESCE(p.barista_level, 1) AS barista_level,
                COALESCE(NULLIF(p.account_tier, ''), INITCAP(COALESCE(u.tier, 'trial'))) AS account_tier,
                COALESCE(p.is_private_account, FALSE) AS is_private_account,
                b.created_at AS edge_created_at
         FROM user_follows a
         INNER JOIN user_follows b
           ON b.follower_id = a.following_id AND b.following_id = $2 AND b.status::text = 'ACCEPTED'
         INNER JOIN users u ON u.id = a.following_id
         LEFT JOIN user_profile_projections p ON p.user_id = a.following_id
         WHERE a.follower_id = $1 AND a.status::text = 'ACCEPTED'
         {cursor_clause}
         ORDER BY b.created_at DESC, a.following_id DESC
         LIMIT {limit_param}"
    );
    let mut query = sqlx::query_as::<_, FollowEdgeUserRow>(&sql)
        .bind(viewer_id)
        .bind(target_id);
    if let Some(c) = cursor {
        query = query.bind(c.created_at).bind(c.user_id);
    }
    Ok(query.bind(limit).fetch_all(pool).await?)
}
