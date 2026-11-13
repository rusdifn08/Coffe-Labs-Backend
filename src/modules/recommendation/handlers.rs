use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, OptionalAuthUser},
        AppState,
    },
    modules::recommendation::{
        dto::{ForYouFeedQuery, MarkSeenRequest, TrendingFeedQuery},
        feed_ranker_service, for_you_ranker,
    },
};

fn parse_uid(raw: &str) -> AppResult<Uuid> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest("User ID tidak valid.".into()))
}

pub async fn get_for_you_feed(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<ForYouFeedQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = for_you_ranker::get_for_you_feed(
        &state.pool,
        &state.redis.active_pool(),
        &state.taste_profiler,
        viewer_id,
        query,
    )
    .await?;
    Ok(ApiResponse::ok("Feed For You berhasil dipersonalisasi.", data))
}

pub async fn get_trending_feed(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<TrendingFeedQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = feed_ranker_service::get_trending_feed(
        &state.pool,
        &state.redis.active_pool(),
        viewer_id,
        query.limit.unwrap_or(25),
    )
    .await?;
    Ok(ApiResponse::ok("Trending feed Coffee Labs berhasil dimuat.", data))
}

pub async fn get_recommended_clubs(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = feed_ranker_service::get_recommended_clubs(
        &state.pool,
        &state.taste_profiler,
        viewer_id,
        8,
    )
    .await?;
    Ok(ApiResponse::ok("Rekomendasi club sesuai selera berhasil dimuat.", data))
}

pub async fn mark_feed_seen(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<MarkSeenRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    feed_ranker_service::mark_seen_best_effort(
        &state.redis.active_pool(),
        user_id,
        &req.thread_ids,
    )
    .await;

    if !req.engaged_thread_ids.is_empty() {
        let texts: Vec<String> = sqlx::query_scalar(
            "SELECT body_text FROM social_threads WHERE id = ANY($1) LIMIT 10",
        )
        .bind(&req.engaged_thread_ids)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default();
        if !texts.is_empty() {
            state
                .taste_profiler
                .record_signal_text_best_effort(user_id, &texts.join(" "))
                .await;
        }
    }

    Ok(ApiResponse::ok(
        "Sinyal feed & preferensi rasa berhasil diperbarui.",
        json!({ "seen_count": req.thread_ids.len() }),
    ))
}
