use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::modules::social_feed::{
    cursor::DirectoryCursor, error::SocialFeedError, models::ProfileDirectoryRow,
};

pub async fn fetch_follow_relationship(
    pool: &PgPool,
    viewer_id: Uuid,
    target_user_id: Uuid,
) -> Result<(bool, bool), SocialFeedError> {
    if viewer_id == target_user_id {
        return Ok((false, false));
    }

    let (is_followed_by_viewer, is_following_viewer): (bool, bool) = sqlx::query_as(
        r#"SELECT
            EXISTS(SELECT 1 FROM user_follows WHERE follower_id = $1 AND following_id = $2 AND status = 'ACCEPTED'),
            EXISTS(SELECT 1 FROM user_follows WHERE follower_id = $2 AND following_id = $1 AND status = 'ACCEPTED')"#,
    )
    .bind(viewer_id)
    .bind(target_user_id)
    .fetch_one(pool)
    .await?;

    Ok((is_followed_by_viewer, is_following_viewer))
}

pub async fn toggle_follow_atomic(
    pool: &PgPool,
    follower_id: Uuid,
    following_id: Uuid,
) -> Result<(bool, i32), SocialFeedError> {
    if follower_id == following_id {
        return Err(SocialFeedError::SelfFollowForbidden);
    }

    let mut tx = pool.begin().await?;
    let deleted_status: Option<String> = sqlx::query_scalar(
        "DELETE FROM user_follows WHERE follower_id = $1 AND following_id = $2 RETURNING status::text",
    )
    .bind(follower_id)
    .bind(following_id)
    .fetch_optional(&mut *tx)
    .await?;

    let is_now_following = if let Some(prev_status) = deleted_status {
        let col = if prev_status == "ACCEPTED" { "followers_count" } else { "pending_follow_requests_count" };
        if prev_status == "ACCEPTED" {
            let sql_f = "UPDATE user_profile_projections SET following_count = GREATEST(following_count - 1, 0), updated_at = NOW() WHERE user_id = $1";
            sqlx::query(sql_f).bind(follower_id).execute(&mut *tx).await?;
        }
        let sql_t = format!("UPDATE user_profile_projections SET {col} = GREATEST({col} - 1, 0), updated_at = NOW() WHERE user_id = $1");
        sqlx::query(&sql_t).bind(following_id).execute(&mut *tx).await?;
        false
    } else {
        let inserted: Option<String> = sqlx::query_scalar(
            "INSERT INTO user_follows (follower_id, following_id, status, accepted_at) VALUES ($1, $2, 'ACCEPTED'::follow_edge_status, NOW()) ON CONFLICT DO NOTHING RETURNING status::text",
        )
        .bind(follower_id)
        .bind(following_id)
        .fetch_optional(&mut *tx)
        .await?;

        if inserted.is_some() {
            sqlx::query("UPDATE user_profile_projections SET following_count = following_count + 1, updated_at = NOW() WHERE user_id = $1")
                .bind(follower_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE user_profile_projections SET followers_count = followers_count + 1, updated_at = NOW() WHERE user_id = $1")
                .bind(following_id)
                .execute(&mut *tx)
                .await?;
        }
        true
    };

    let followers_count: i32 = sqlx::query_scalar(
        "SELECT followers_count FROM user_profile_projections WHERE user_id = $1",
    )
    .bind(following_id)
    .fetch_optional(&mut *tx)
    .await?
    .unwrap_or(0);

    tx.commit().await?;
    Ok((is_now_following, followers_count))
}

pub async fn list_directory_profiles(
    pool: &PgPool,
    search: Option<&str>,
    cursor: Option<DirectoryCursor>,
    limit: i64,
) -> Result<Vec<ProfileDirectoryRow>, SocialFeedError> {
    let pattern = format!("%{}%", search.unwrap_or("").trim());
    let rows = match cursor {
        Some(c) => {
            sqlx::query_as::<_, ProfileDirectoryRow>(
                r#"SELECT user_id, username, display_name, bio, avatar_url, account_tier,
                          total_xp, barista_level, barista_rank_title, followers_count,
                          total_threads_count, recent_brew_snapshot, updated_at
                   FROM user_profile_projections
                   WHERE ($1 = '%%' OR username ILIKE $1 OR display_name ILIKE $1)
                     AND (total_xp < $2 OR (total_xp = $2 AND user_id < $3))
                   ORDER BY total_xp DESC, user_id DESC
                   LIMIT $4"#,
            )
            .bind(&pattern)
            .bind(c.total_xp)
            .bind(c.user_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, ProfileDirectoryRow>(
                r#"SELECT user_id, username, display_name, bio, avatar_url, account_tier,
                          total_xp, barista_level, barista_rank_title, followers_count,
                          total_threads_count, recent_brew_snapshot, updated_at
                   FROM user_profile_projections
                   WHERE ($1 = '%%' OR username ILIKE $1 OR display_name ILIKE $1)
                   ORDER BY total_xp DESC, user_id DESC
                   LIMIT $2"#,
            )
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

pub async fn list_follow_connections(
    pool: &PgPool,
    target_user_id: Uuid,
    fetch_followers: bool,
    limit: i64,
) -> Result<Vec<ProfileDirectoryRow>, SocialFeedError> {
    let sql = if fetch_followers {
        r#"SELECT p.user_id, p.username, p.display_name, p.bio, p.avatar_url, p.account_tier,
                  p.total_xp, p.barista_level, p.barista_rank_title, p.followers_count,
                  p.total_threads_count, p.recent_brew_snapshot, p.updated_at
           FROM user_follows f
           INNER JOIN user_profile_projections p ON p.user_id = f.follower_id
           WHERE f.following_id = $1 AND f.status = 'ACCEPTED'
           ORDER BY f.created_at DESC
           LIMIT $2"#
    } else {
        r#"SELECT p.user_id, p.username, p.display_name, p.bio, p.avatar_url, p.account_tier,
                  p.total_xp, p.barista_level, p.barista_rank_title, p.followers_count,
                  p.total_threads_count, p.recent_brew_snapshot, p.updated_at
           FROM user_follows f
           INNER JOIN user_profile_projections p ON p.user_id = f.following_id
           WHERE f.follower_id = $1 AND f.status = 'ACCEPTED'
           ORDER BY f.created_at DESC
           LIMIT $2"#
    };

    let rows = sqlx::query_as::<_, ProfileDirectoryRow>(sql)
        .bind(target_user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn fetch_viewer_followed_ids(
    pool: &PgPool,
    viewer_id: Uuid,
    candidate_ids: &[Uuid],
) -> Result<HashSet<Uuid>, SocialFeedError> {
    if candidate_ids.is_empty() {
        return Ok(HashSet::new());
    }
    let rows = sqlx::query_scalar::<_, Uuid>(
        "SELECT following_id FROM user_follows WHERE follower_id = $1 AND status = 'ACCEPTED' AND following_id = ANY($2::uuid[])",
    )
    .bind(viewer_id)
    .bind(candidate_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().collect())
}
