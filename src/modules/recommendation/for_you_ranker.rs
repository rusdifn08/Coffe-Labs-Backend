use chrono::Utc;
use fred::clients::RedisPool;
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::{
    core::error::AppError,
    modules::{
        recommendation::{
            dto::{ForYouFeedQuery, ForYouFeedResponseDto, RecommendedThreadItemDto},
            feed_ranker_service::{
                live_gravity_for_row, load_seen_ids, mark_seen_best_effort, resolve_candidate_taste,
            },
            recommendation_repo::{self, ScoredThreadCandidateRow},
            scoring_math::{
                compute_for_you_score, cosine_similarity, neutral_taste_vector, ForYouScoreInput,
            },
            taste_profiler_service::TasteProfilerService,
        },
        social_feed::services::thread_hydrator,
    },
};

struct RankedCandidate {
    row: ScoredThreadCandidateRow,
    score: f64,
    taste_match_pct: u8,
    reason_badge: String,
    is_trending: bool,
}

fn matches_method_filter(item: &ScoredThreadCandidateRow, filter: &str) -> bool {
    if item.body_text.to_ascii_lowercase().contains(filter) {
        return true;
    }
    item.attachment_payload
        .as_ref()
        .map(|v| v.to_string().to_ascii_lowercase().contains(filter))
        .unwrap_or(false)
}

pub async fn get_for_you_feed(
    pg: &PgPool,
    redis: &Option<RedisPool>,
    taste_profiler: &TasteProfilerService,
    viewer_id: Option<Uuid>,
    query: ForYouFeedQuery,
) -> Result<ForYouFeedResponseDto, AppError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 40) as usize;
    let offset = query.cursor_offset.unwrap_or(0);

    let (trend_res, social_res, disc_res, seen_ids) = match viewer_id {
        Some(uid) => {
            let (t, s, d, seen) = tokio::join!(
                recommendation_repo::fetch_trending_candidates(pg, Some(uid), 60),
                recommendation_repo::fetch_social_circle_candidates(pg, uid, 40),
                recommendation_repo::fetch_recipe_discovery_candidates(pg, Some(uid), 40),
                load_seen_ids(redis, Some(uid)),
            );
            (t?, s?, d?, seen)
        }
        None => {
            let (t, d) = tokio::join!(
                recommendation_repo::fetch_trending_candidates(pg, None, 50),
                recommendation_repo::fetch_recipe_discovery_candidates(pg, None, 40),
            );
            (t?, Vec::new(), d?, HashSet::new())
        }
    };

    let (user_taste, following_set, my_clubs) = match viewer_id {
        Some(uid) => {
            let (taste, (following, clubs)) = tokio::join!(
                taste_profiler.get_or_load_user_taste(uid),
                recommendation_repo::fetch_viewer_social_sets(pg, uid),
            );
            (taste, following, clubs)
        }
        None => (neutral_taste_vector(), HashSet::new(), HashSet::new()),
    };

    let method_filter = query
        .method_filter
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("ALL"))
        .map(str::to_ascii_lowercase);

    let mut dedup: HashMap<Uuid, ScoredThreadCandidateRow> = HashMap::new();
    for item in trend_res.into_iter().chain(social_res).chain(disc_res) {
        if let Some(filter) = &method_filter {
            if !matches_method_filter(&item, filter) {
                continue;
            }
        }
        dedup.entry(item.id).or_insert(item);
    }

    let now = Utc::now();
    let mut scored: Vec<RankedCandidate> = dedup
        .into_values()
        .map(|row| {
            let p_taste = resolve_candidate_taste(&row);
            let cos = cosine_similarity(&user_taste, &p_taste);
            let live_trend = live_gravity_for_row(&row, now);
            let is_shared_club = row
                .community_id
                .map(|cid| my_clubs.contains(&cid))
                .unwrap_or(false);
            let (score, badge) = compute_for_you_score(&ForYouScoreInput {
                trending_gravity: live_trend,
                taste_cosine: cos,
                is_following_author: following_set.contains(&row.author_id),
                is_shared_club,
                is_own_post: viewer_id == Some(row.author_id),
                is_seen: seen_ids.contains(&row.id),
                has_recipe_card: row.attachment_type.as_deref() == Some("RECIPE_CARD"),
            });
            RankedCandidate {
                taste_match_pct: (cos * 100.0).round().clamp(0.0, 99.0) as u8,
                reason_badge: badge,
                is_trending: live_trend > 0.18,
                score,
                row,
            }
        })
        .collect();

    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut author_counts: HashMap<Uuid, usize> = HashMap::new();
    for item in &mut scored {
        let count = author_counts.entry(item.row.author_id).or_insert(0);
        if *count > 0 {
            item.score -= 0.18 * (*count as f64);
        }
        *count += 1;
    }
    scored.sort_by(|a, b| b.score.total_cmp(&a.score));

    let total = scored.len();
    let page: Vec<RankedCandidate> = scored.into_iter().skip(offset).take(limit).collect();
    let has_more = offset + page.len() < total;
    let next_offset = if has_more { Some(offset + page.len()) } else { None };

    if let Some(uid) = viewer_id {
        let delivered_ids: Vec<Uuid> = page.iter().map(|p| p.row.id).collect();
        mark_seen_best_effort(redis, uid, &delivered_ids).await;
    }

    let author_rows = page.iter().map(|p| p.row.to_author_row()).collect();
    let hydrated = thread_hydrator::hydrate_threads_batch(pg, viewer_id, author_rows).await?;

    let items = hydrated
        .into_iter()
        .zip(page)
        .map(|(thread, meta)| RecommendedThreadItemDto {
            thread,
            relevance_score: (meta.score * 100.0).round() / 100.0,
            taste_match_pct: meta.taste_match_pct,
            reason_badge: meta.reason_badge,
            is_trending: meta.is_trending,
            community_id: meta.row.community_id,
            community_name: meta.row.community_name,
        })
        .collect();

    Ok(ForYouFeedResponseDto {
        items,
        user_top_tastes: TasteProfilerService::top_taste_labels(&user_taste, 3),
        next_offset,
        has_more,
    })
}
