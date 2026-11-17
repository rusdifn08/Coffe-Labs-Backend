use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, OptionalAuthUser},
        AppState,
    },
    modules::social_feed::{
        dto::{DirectorySearchQuery, UpdateMyProfileRequest},
        services::profile_directory_service,
    },
};

pub async fn explore_profiles(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<DirectorySearchQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().map(|c| c.sub);
    let redis_pool = state.redis.active_pool();
    let data = profile_directory_service::explore_directory_cached(
        &state.pool,
        &state.caches,
        redis_pool.as_ref(),
        viewer_id,
        query.q.as_deref(),
        query.cursor.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await?;
    Ok(ApiResponse::ok("Direktori barista berhasil dimuat.", data))
}

pub async fn get_my_profile(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .social_profile_cqrs
        .get_materialized_profile(&state.pool, Some(claims.sub), "me")
        .await?;
    Ok(ApiResponse::ok("Profil saya berhasil dimuat.", data))
}

pub async fn get_profile(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(target): Path<String>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().map(|c| c.sub);
    let data = state
        .social_profile_cqrs
        .get_materialized_profile(&state.pool, viewer_id, &target)
        .await?;
    Ok(ApiResponse::ok("Profil barista berhasil dimuat.", data))
}

pub async fn update_my_profile(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(req): Json<UpdateMyProfileRequest>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .social_profile_cqrs
        .update_my_profile(&state.pool, &state.chat_moderation, claims.sub, req)
        .await?;
    state.caches.directory_l1.invalidate_all();
    Ok(ApiResponse::ok("Profil Anda berhasil diperbarui.", data))
}

pub async fn toggle_follow(
    AuthUser(claims): AuthUser,
    Path(target): Path<String>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .social_profile_cqrs
        .toggle_follow(&state.pool, claims.sub, &target)
        .await?;
    state.caches.viewer_followed_l1.invalidate(&claims.sub).await;
    state.caches.directory_l1.invalidate_all();
    let msg = if data.is_followed_by_viewer {
        "Berhasil mengikuti barista ini."
    } else {
        "Berhenti mengikuti barista ini."
    };
    Ok(ApiResponse::ok(msg, data))
}

pub async fn list_followers(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(target): Path<String>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().map(|c| c.sub);
    let data = profile_directory_service::list_connections(
        &state.pool,
        viewer_id,
        &target,
        true,
        50,
    )
    .await?;
    Ok(ApiResponse::ok("Daftar pengikut berhasil dimuat.", data))
}

pub async fn list_following(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(target): Path<String>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().map(|c| c.sub);
    let data = profile_directory_service::list_connections(
        &state.pool,
        viewer_id,
        &target,
        false,
        50,
    )
    .await?;
    Ok(ApiResponse::ok("Daftar yang diikuti berhasil dimuat.", data))
}
