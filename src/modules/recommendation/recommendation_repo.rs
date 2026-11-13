use sqlx::PgPool;
use std::collections::HashSet;
use uuid::Uuid;

use crate::modules::communities::models::CommunityDiscoverRow;
pub use crate::modules::recommendation::recommendation_models::ScoredThreadCandidateRow;

const SELECT_CANDIDATE_PREFIX: &str = r#"
SELECT t.id, t.author_id, p.username AS author_username, p.display_name AS author_display_name,
       p.avatar_url AS author_avatar_url, p.account_tier AS author_tier,
       p.barista_level AS author_barista_level, p.barista_rank_title AS author_barista_rank_title,
       t.kind::text AS kind, t.parent_thread_id, t.root_thread_id, t.quoted_thread_id,
       t.depth_level, t.body_text, t.has_media, t.media_urls, t.attachment_type,
       t.attachment_payload, t.likes_count, t.replies_count, t.reposts_count,
       t.try_recipe_count, t.is_pinned_on_profile, t.created_at,
       t.community_id, c.name AS community_name, t.trending_gravity_score, t.taste_feature_vector
FROM social_threads t
INNER JOIN user_profile_projections p ON p.user_id = t.author_id
LEFT JOIN coffee_communities c ON c.id = t.community_id
"#;

const VISIBILITY_PREDICATE: &str = r#"
t.is_deleted = FALSE AND t.depth_level = 0
AND (
    t.community_id IS NULL
    OR c.privacy_level = 'PUBLIC'
    OR ($1::uuid IS NOT NULL AND EXISTS (
        SELECT 1 FROM community_members cm WHERE cm.community_id = t.community_id AND cm.user_id = $1
    ))
)
"#;

pub async fn fetch_trending_candidates(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<ScoredThreadCandidateRow>, sqlx::Error> {
    let sql = format!(
        "{SELECT_CANDIDATE_PREFIX} WHERE {VISIBILITY_PREDICATE}
         ORDER BY t.trending_gravity_score DESC, t.likes_count + t.try_recipe_count DESC, t.id DESC
         LIMIT $2"
    );
    sqlx::query_as(&sql)
        .bind(viewer_id)
        .bind(limit)
        .fetch_all(pg)
        .await
}

pub async fn fetch_social_circle_candidates(
    pg: &PgPool,
    viewer_id: Uuid,
    limit: i64,
) -> Result<Vec<ScoredThreadCandidateRow>, sqlx::Error> {
    let sql = format!(
        "{SELECT_CANDIDATE_PREFIX} WHERE {VISIBILITY_PREDICATE}
         AND (
             EXISTS (SELECT 1 FROM user_follows f WHERE f.follower_id = $1 AND f.following_id = t.author_id AND f.status = 'ACCEPTED')
             OR (t.community_id IS NOT NULL AND EXISTS (SELECT 1 FROM community_members cm WHERE cm.community_id = t.community_id AND cm.user_id = $1))
         )
         ORDER BY t.id DESC LIMIT $2"
    );
    sqlx::query_as(&sql)
        .bind(Some(viewer_id))
        .bind(limit)
        .fetch_all(pg)
        .await
}

pub async fn fetch_recipe_discovery_candidates(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<ScoredThreadCandidateRow>, sqlx::Error> {
    let sql = format!(
        "{SELECT_CANDIDATE_PREFIX} WHERE {VISIBILITY_PREDICATE}
         AND t.created_at > NOW() - INTERVAL '30 days'
         ORDER BY CASE WHEN t.attachment_type IS NOT NULL THEN 0 ELSE 1 END ASC, t.id DESC
         LIMIT $2"
    );
    sqlx::query_as(&sql)
        .bind(viewer_id)
        .bind(limit)
        .fetch_all(pg)
        .await
}

pub async fn fetch_posts_by_ids(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    ids: &[Uuid],
) -> Result<Vec<ScoredThreadCandidateRow>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "{SELECT_CANDIDATE_PREFIX} WHERE {VISIBILITY_PREDICATE} AND t.id = ANY($2)"
    );
    sqlx::query_as(&sql)
        .bind(viewer_id)
        .bind(ids)
        .fetch_all(pg)
        .await
}

pub async fn fetch_viewer_social_sets(
    pg: &PgPool,
    viewer_id: Uuid,
) -> (HashSet<Uuid>, HashSet<Uuid>) {
    let following_fut = sqlx::query_scalar::<_, Uuid>(
        "SELECT following_id FROM user_follows WHERE follower_id = $1 AND status = 'ACCEPTED'",
    )
    .bind(viewer_id)
    .fetch_all(pg);
    let clubs_fut = sqlx::query_scalar::<_, Uuid>(
        "SELECT community_id FROM community_members WHERE user_id = $1",
    )
    .bind(viewer_id)
    .fetch_all(pg);

    let (f_res, c_res) = tokio::join!(following_fut, clubs_fut);
    (
        f_res.unwrap_or_default().into_iter().collect(),
        c_res.unwrap_or_default().into_iter().collect(),
    )
}

pub async fn fetch_candidate_clubs_for_recommendation(
    pg: &PgPool,
    viewer_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<CommunityDiscoverRow>, sqlx::Error> {
    sqlx::query_as::<_, CommunityDiscoverRow>(
        r#"SELECT c.id, c.handle_slug, c.name, c.description, c.avatar_url, c.banner_url,
                  c.privacy_level::text AS privacy_level, c.owner_id, c.focus_tags,
                  c.taste_feature_vector, c.members_count, c.threads_count,
                  c.shared_recipes_count, c.total_club_xp,
                  cm.role::text AS viewer_role, c.created_at
           FROM coffee_communities c
           LEFT JOIN community_members cm ON cm.community_id = c.id AND cm.user_id = $1
           WHERE c.privacy_level <> 'SECRET_VAULT' AND cm.user_id IS NULL
           ORDER BY c.members_count DESC, c.total_club_xp DESC
           LIMIT $2"#,
    )
    .bind(viewer_id)
    .bind(limit)
    .fetch_all(pg)
    .await
}
