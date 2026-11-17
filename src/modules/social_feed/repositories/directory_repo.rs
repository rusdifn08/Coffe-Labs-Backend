use sqlx::PgPool;

use crate::modules::social_feed::{
    cursor::DirectoryCursor, error::SocialFeedError, models::ProfileDirectoryRow,
};

pub fn normalize_search_query(raw: Option<&str>) -> String {
    raw.unwrap_or("")
        .trim()
        .trim_start_matches('@')
        .to_lowercase()
}

pub async fn search_directory_profiles(
    pool: &PgPool,
    search: Option<&str>,
    cursor: Option<DirectoryCursor>,
    limit: i64,
) -> Result<Vec<ProfileDirectoryRow>, SocialFeedError> {
    let clean = normalize_search_query(search);
    let contains_pattern = format!("%{clean}%");
    let prefix_pattern = format!("{clean}%");

    let rows = match cursor {
        Some(c) => {
            sqlx::query_as::<_, ProfileDirectoryRow>(
                r#"SELECT user_id, username, display_name, bio, avatar_url, account_tier,
                          total_xp, barista_level, barista_rank_title, followers_count,
                          total_threads_count, recent_brew_snapshot
                   FROM user_profile_projections
                   WHERE ($1 = '%%' OR LOWER(username) LIKE $1 OR LOWER(display_name) LIKE $1)
                     AND (total_xp < $2 OR (total_xp = $2 AND user_id < $3))
                   ORDER BY total_xp DESC, user_id DESC
                   LIMIT $4"#,
            )
            .bind(&contains_pattern)
            .bind(c.total_xp)
            .bind(c.user_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            let mut projected = sqlx::query_as::<_, ProfileDirectoryRow>(
                r#"SELECT user_id, username, display_name, bio, avatar_url, account_tier,
                          total_xp, barista_level, barista_rank_title, followers_count,
                          total_threads_count, recent_brew_snapshot
                   FROM user_profile_projections
                   WHERE ($1 = '%%' OR LOWER(username) LIKE $1 OR LOWER(display_name) LIKE $1)
                   ORDER BY
                       CASE
                           WHEN $2 = '' THEN 0
                           WHEN LOWER(username) = $2 THEN 0
                           WHEN LOWER(username) LIKE $3 THEN 1
                           WHEN LOWER(display_name) LIKE $3 THEN 2
                           ELSE 3
                       END ASC,
                       total_xp DESC,
                       user_id DESC
                   LIMIT $4"#,
            )
            .bind(&contains_pattern)
            .bind(&clean)
            .bind(&prefix_pattern)
            .bind(limit)
            .fetch_all(pool)
            .await?;

            if (projected.len() as i64) < limit {
                let remaining = limit - (projected.len() as i64);
                let unprojected = sqlx::query_as::<_, ProfileDirectoryRow>(
                    r#"SELECT u.id AS user_id,
                              LEFT(u.username, 40) AS username,
                              LEFT(COALESCE(NULLIF(u.full_name, ''), u.username), 80) AS display_name,
                              COALESCE(u.bio, 'Barista & V60 Enthusiast di Coffee Labs') AS bio,
                              u.avatar_url,
                              INITCAP(COALESCE(u.tier, 'trial')) AS account_tier,
                              0::bigint AS total_xp,
                              1::int AS barista_level,
                              'Home Brewer'::varchar(50) AS barista_rank_title,
                              0::int AS followers_count,
                              0::int AS total_threads_count,
                              NULL::jsonb AS recent_brew_snapshot
                       FROM users u
                       WHERE NOT EXISTS (SELECT 1 FROM user_profile_projections p WHERE p.user_id = u.id)
                         AND LOWER(COALESCE(u.status, 'approved')) IN ('approved', 'active')
                         AND ($1 = '%%' OR LOWER(u.username) LIKE $1 OR LOWER(u.full_name) LIKE $1)
                       ORDER BY u.updated_at DESC
                       LIMIT $2"#,
                )
                .bind(&contains_pattern)
                .bind(remaining)
                .fetch_all(pool)
                .await
                .unwrap_or_default();
                projected.extend(unprojected);
            }
            projected
        }
    };

    Ok(rows)
}
