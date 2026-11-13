use chrono::{DateTime, Utc};
use fred::{
    clients::RedisPool,
    interfaces::{KeysInterface, SetsInterface, SortedSetsInterface},
};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
    core::error::AppError,
    modules::{
        communities::services::club_discovery_service::summary_from_discover,
        recommendation::{
            dto::{
                RecommendedClubsResponseDto, RecommendedThreadItemDto, TrendingFeedResponseDto,
            },
            recommendation_repo::{self, ScoredThreadCandidateRow},
            scoring_math::{
                compute_trending_gravity_score, cosine_similarity, extract_16d_vector_from_text,
                neutral_taste_vector, vec_from_slice,
            },
            taste_profiler_service::TasteProfilerService,
            trending_worker::FEED_TRENDING_ZSET_KEY,
        },
        social_feed::services::thread_hydrator,
    },
};

const SEEN_TTL_SECS: i64 = 48 * 3600;

fn seen_key(user_id: Uuid) -> String {
    format!("feed:seen:{user_id}")
}

pub fn resolve_candidate_taste(row: &ScoredThreadCandidateRow) -> [f32; 16] {
    if row.taste_feature_vector.iter().any(|v| *v > 1e-4) {
        return vec_from_slice(&row.taste_feature_vector);
    }
    extract_16d_vector_from_text(&row.body_text)
}

pub fn live_gravity_for_row(row: &ScoredThreadCandidateRow, now: DateTime<Utc>) -> f64 {
    let age_hours = (now - row.created_at).num_seconds().max(0) as f64 / 3600.0;
    compute_trending_gravity_score(
        row.likes_count,
        row.replies_count,
        row.reposts_count,
        row.try_recipe_count,
        row.has_media,
        row.attachment_type.is_some(),
        age_hours,
    )
    .max(row.trending_gravity_score)
}

pub async fn load_seen_ids(redis: &Option<RedisPool>, user_id: Option<Uuid>) -> HashSet<Uuid> {
    let (Some(pool), Some(uid)) = (redis, user_id) else {
        return HashSet::new();
    };
    let members: Result<Vec<String>, _> = pool.smembers(seen_key(uid)).await;
    members
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| Uuid::parse_str(&s).ok())
        .collect()
}

pub async fn mark_seen_best_effort(redis: &Option<RedisPool>, user_id: Uuid, ids: &[Uuid]) {
    let Some(pool) = redis else { return };
    if ids.is_empty() {
        return;
    }
    let key = seen_key(user_id);
    let str_ids: Vec<String> = ids.iter().map(ToString::to_string).collect();
    let _: Result<i64, _> = pool.sadd(&key, str_ids).await;
    let _: Result<bool, _> = pool.expire(&key, SEEN_TTL_SECS).await;
}

pub async fn get_trending_feed(
    pg: &PgPool,
    redis: &Option<RedisPool>,
    viewer_id: Option<Uuid>,
    limit: i64,
) -> Result<TrendingFeedResponseDto, AppError> {
    let clamped = limit.clamp(1, 50);
    let mut rows = Vec::new();
    if let Some(pool) = redis {
        let z_ids: Vec<String> = pool
            .zrevrange(FEED_TRENDING_ZSET_KEY, 0, clamped - 1, false)
            .await
            .unwrap_or_default();
        let parsed: Vec<Uuid> = z_ids.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect();
        if !parsed.is_empty() {
            let db_rows = recommendation_repo::fetch_posts_by_ids(pg, viewer_id, &parsed).await?;
            let by_id: HashMap<Uuid, ScoredThreadCandidateRow> =
                db_rows.into_iter().map(|r| (r.id, r)).collect();
            for uid in parsed {
                if let Some(item) = by_id.get(&uid) {
                    rows.push(item.clone());
                }
            }
        }
    }
    if rows.is_empty() {
        rows = recommendation_repo::fetch_trending_candidates(pg, viewer_id, clamped).await?;
    }

    let now = Utc::now();
    let meta: Vec<(f64, Option<Uuid>, Option<String>)> = rows
        .iter()
        .map(|r| (live_gravity_for_row(r, now), r.community_id, r.community_name.clone()))
        .collect();

    let author_rows = rows.iter().map(ScoredThreadCandidateRow::to_author_row).collect();
    let hydrated = thread_hydrator::hydrate_threads_batch(pg, viewer_id, author_rows).await?;

    let items = hydrated
        .into_iter()
        .zip(meta)
        .enumerate()
        .map(|(idx, (thread, (score, cid, cname)))| RecommendedThreadItemDto {
            thread,
            relevance_score: score,
            taste_match_pct: 0,
            reason_badge: if idx < 3 {
                format!("🔥 Top #{} Trending Brew", idx + 1)
            } else {
                "🔥 Trending di Coffee Labs".to_string()
            },
            is_trending: true,
            community_id: cid,
            community_name: cname,
        })
        .collect();

    Ok(TrendingFeedResponseDto {
        items,
        updated_at: now,
    })
}

pub async fn get_recommended_clubs(
    pg: &PgPool,
    taste_profiler: &TasteProfilerService,
    viewer_id: Option<Uuid>,
    limit: i64,
) -> Result<RecommendedClubsResponseDto, AppError> {
    let clamped = limit.clamp(1, 20);
    let user_taste = match viewer_id {
        Some(uid) => taste_profiler.get_or_load_user_taste(uid).await,
        None => neutral_taste_vector(),
    };

    let candidates =
        recommendation_repo::fetch_candidate_clubs_for_recommendation(pg, viewer_id, 40).await?;
    let mut ranked: Vec<_> = candidates
        .into_iter()
        .map(|row| {
            let club_vec = vec_from_slice(&row.taste_feature_vector);
            let cos = cosine_similarity(&user_taste, &club_vec);
            let pop_bonus = ((1.0 + row.members_count as f64).ln() * 0.08).min(0.35);
            let total_score = 0.72 * (cos as f64) + pop_bonus;
            let mut dto = summary_from_discover(row);
            dto.match_score_pct = Some((cos * 100.0).clamp(15.0, 99.0));
            (total_score, dto)
        })
        .collect();

    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    ranked.truncate(clamped as usize);

    Ok(RecommendedClubsResponseDto {
        items: ranked.into_iter().map(|(_, dto)| dto).collect(),
        user_top_tastes: TasteProfilerService::top_taste_labels(&user_taste, 3),
    })
}
