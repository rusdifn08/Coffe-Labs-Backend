use chrono::{DateTime, Utc};
use fred::{clients::RedisPool, interfaces::SortedSetsInterface};
use sqlx::{FromRow, PgPool};
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

use crate::modules::recommendation::scoring_math::compute_trending_gravity_score;

pub const FEED_TRENDING_ZSET_KEY: &str = "feed:trending:global";
const INITIAL_DELAY_SECS: u64 = 15;
const WORKER_INTERVAL_SECS: u64 = 300;
const ZSET_MAX_ITEMS: i64 = 200;

#[derive(FromRow)]
struct ActiveThreadStatRow {
    id: Uuid,
    likes_count: i32,
    replies_count: i32,
    reposts_count: i32,
    try_recipe_count: i32,
    has_media: bool,
    has_attachment: bool,
    created_at: DateTime<Utc>,
    is_public_for_zset: bool,
}

pub fn spawn_trending_worker(pg: PgPool, redis: Option<RedisPool>) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(INITIAL_DELAY_SECS)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(WORKER_INTERVAL_SECS));
        info!("TrendingFeedWorker aktif (interval 300s, ZSET cap 200)");
        loop {
            ticker.tick().await;
            if let Err(err) = recompute_trending_batch(&pg, &redis).await {
                warn!(error = %err, "TrendingFeedWorker cycle gagal");
            }
        }
    });
}

pub async fn recompute_trending_batch(
    pg: &PgPool,
    redis: &Option<RedisPool>,
) -> Result<usize, sqlx::Error> {
    let rows = sqlx::query_as::<_, ActiveThreadStatRow>(
        r#"SELECT t.id, t.likes_count, t.replies_count, t.reposts_count,
                  t.try_recipe_count, t.has_media, (t.attachment_type IS NOT NULL) AS has_attachment,
                  t.created_at,
                  (t.community_id IS NULL OR c.privacy_level = 'PUBLIC') AS is_public_for_zset
           FROM social_threads t
           LEFT JOIN coffee_communities c ON c.id = t.community_id
           WHERE t.is_deleted = FALSE
             AND t.depth_level = 0
             AND t.created_at > NOW() - INTERVAL '14 days'
           ORDER BY t.id DESC
           LIMIT 500"#,
    )
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        return Ok(0);
    }

    let now = Utc::now();
    let mut ids = Vec::with_capacity(rows.len());
    let mut scores = Vec::with_capacity(rows.len());
    let mut zset_items: Vec<(f64, String)> = Vec::new();

    for r in rows {
        let age_hours = (now - r.created_at).num_seconds().max(0) as f64 / 3600.0;
        let g = compute_trending_gravity_score(
            r.likes_count,
            r.replies_count,
            r.reposts_count,
            r.try_recipe_count,
            r.has_media,
            r.has_attachment,
            age_hours,
        );
        ids.push(r.id);
        scores.push(g);
        if r.is_public_for_zset {
            zset_items.push((g, r.id.to_string()));
        }
    }

    let updated = ids.len();
    sqlx::query(
        r#"UPDATE social_threads t
           SET trending_gravity_score = v.score
           FROM UNNEST($1::uuid[], $2::double precision[]) AS v(id, score)
           WHERE t.id = v.id"#,
    )
    .bind(&ids)
    .bind(&scores)
    .execute(pg)
    .await?;

    if let Some(pool) = redis {
        if !zset_items.is_empty() {
            zset_items.sort_by(|a, b| b.0.total_cmp(&a.0));
            zset_items.truncate(ZSET_MAX_ITEMS as usize);
            let _: Result<i64, _> = pool
                .zadd(FEED_TRENDING_ZSET_KEY, None, None, false, false, zset_items)
                .await;
            let _: Result<i64, _> = pool
                .zremrangebyrank(FEED_TRENDING_ZSET_KEY, 0, -(ZSET_MAX_ITEMS + 1))
                .await;
        }
    }

    Ok(updated)
}
