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
        "SELECT p.user_id, p.username, p.display_name, p.avatar_url,
                p.barista_level, p.account_tier, p.is_private_account,
                f.created_at AS edge_created_at
         FROM user_follows f
         INNER JOIN user_profile_projections p ON p.user_id = f.{join_col}
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
        "SELECT p.user_id, p.username, p.display_name, p.avatar_url,
                p.barista_level, p.account_tier, p.is_private_account,
                b.created_at AS edge_created_at
         FROM user_follows a
         INNER JOIN user_follows b
           ON b.follower_id = a.following_id AND b.following_id = $2 AND b.status = 'ACCEPTED'
         INNER JOIN user_profile_projections p ON p.user_id = a.following_id
         WHERE a.follower_id = $1 AND a.status = 'ACCEPTED'
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
