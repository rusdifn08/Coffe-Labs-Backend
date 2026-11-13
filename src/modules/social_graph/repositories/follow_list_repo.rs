use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::modules::social_graph::{
    error::SocialGraphError,
    models::MiniUserProfileRow,
};

#[derive(FromRow)]
struct BatchEdgeSqlRow {
    target_id: Uuid,
    out_accepted: bool,
    in_accepted: bool,
    out_pending: bool,
}

#[derive(FromRow)]
struct WarmupEdgeRow {
    edge_kind: String,
    peer_id: Uuid,
}

pub async fn fetch_edge_sets_for_warmup(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<(Vec<Uuid>, Vec<Uuid>, Vec<Uuid>), SocialGraphError> {
    let rows = sqlx::query_as::<_, WarmupEdgeRow>(
        "SELECT 'out_acc' AS edge_kind, following_id AS peer_id
         FROM user_follows WHERE follower_id = $1 AND status = 'ACCEPTED'
         UNION ALL
         SELECT 'in_acc' AS edge_kind, follower_id AS peer_id
         FROM user_follows WHERE following_id = $1 AND status = 'ACCEPTED'
         UNION ALL
         SELECT 'out_pen' AS edge_kind, following_id AS peer_id
         FROM user_follows WHERE follower_id = $1 AND status = 'PENDING'",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let (mut following, mut followers, mut pending_out) = (Vec::new(), Vec::new(), Vec::new());
    for row in rows {
        match row.edge_kind.as_str() {
            "out_acc" => following.push(row.peer_id),
            "in_acc" => followers.push(row.peer_id),
            "out_pen" => pending_out.push(row.peer_id),
            _ => {}
        }
    }
    Ok((following, followers, pending_out))
}

pub async fn fetch_mutual_stats_sql(
    pool: &PgPool,
    viewer_id: Uuid,
    target_id: Uuid,
) -> Result<(i64, Vec<Uuid>), SocialGraphError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint
         FROM user_follows a
         INNER JOIN user_follows b
           ON b.follower_id = a.following_id
          AND b.following_id = $2
          AND b.status = 'ACCEPTED'
         WHERE a.follower_id = $1 AND a.status = 'ACCEPTED'",
    )
    .bind(viewer_id)
    .bind(target_id)
    .fetch_one(pool)
    .await?;

    if count == 0 {
        return Ok((0, Vec::new()));
    }

    let sample_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT a.following_id
         FROM user_follows a
         INNER JOIN user_follows b
           ON b.follower_id = a.following_id
          AND b.following_id = $2
          AND b.status = 'ACCEPTED'
         WHERE a.follower_id = $1 AND a.status = 'ACCEPTED'
         ORDER BY b.created_at DESC
         LIMIT 3",
    )
    .bind(viewer_id)
    .bind(target_id)
    .fetch_all(pool)
    .await?;

    Ok((count, sample_ids))
}

pub async fn fetch_mini_profiles_by_ids(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<Vec<MiniUserProfileRow>, SocialGraphError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, MiniUserProfileRow>(
        "SELECT user_id, username, display_name, avatar_url, barista_level
         FROM user_profile_projections WHERE user_id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?)
}

pub async fn fetch_batch_edge_vectors_sql(
    pool: &PgPool,
    viewer_id: Uuid,
    target_ids: &[Uuid],
) -> Result<Vec<(Uuid, bool, bool, bool)>, SocialGraphError> {
    if target_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, BatchEdgeSqlRow>(
        "SELECT t.tid AS target_id,
                EXISTS(SELECT 1 FROM user_follows o WHERE o.follower_id = $1 AND o.following_id = t.tid AND o.status = 'ACCEPTED') AS out_accepted,
                EXISTS(SELECT 1 FROM user_follows i WHERE i.follower_id = t.tid AND i.following_id = $1 AND i.status = 'ACCEPTED') AS in_accepted,
                EXISTS(SELECT 1 FROM user_follows p WHERE p.follower_id = $1 AND p.following_id = t.tid AND p.status = 'PENDING') AS out_pending
         FROM UNNEST($2::uuid[]) AS t(tid)",
    )
    .bind(viewer_id)
    .bind(target_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| (r.target_id, r.out_accepted, r.in_accepted, r.out_pending))
        .collect())
}
