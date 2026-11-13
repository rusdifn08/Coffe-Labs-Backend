use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, OptionalAuthUser},
        AppState,
    },
    modules::communities::{
        dto::{ClubFeedQuery, CreateClubThreadRequest},
        services::club_vault_service,
    },
};

fn parse_uid(raw: &str) -> AppResult<Uuid> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest("User ID tidak valid.".into()))
}

pub async fn get_club_feed(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(community_id): Path<Uuid>,
    Query(query): Query<ClubFeedQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = club_vault_service::get_club_feed(
        &state.pool,
        &state.club_cache,
        viewer_id,
        community_id,
        query,
    )
    .await?;
    Ok(ApiResponse::ok("Feed komunitas berhasil dimuat.", data))
}

pub async fn get_club_recipe_vault(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = club_vault_service::get_club_recipe_vault(
        &state.pool,
        &state.club_cache,
        viewer_id,
        community_id,
    )
    .await?;
    Ok(ApiResponse::ok("Recipe Vault komunitas berhasil dimuat.", data))
}

pub async fn create_club_post(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<CreateClubThreadRequest>,
) -> AppResult<impl IntoResponse> {
    let author_id = parse_uid(&claims.sub)?;
    let data = club_vault_service::create_club_post(
        &state.pool,
        &state.club_cache,
        &state.social_profile_cqrs,
        &state.chat_moderation,
        &state.taste_profiler,
        author_id,
        community_id,
        req,
    )
    .await?;
    Ok(ApiResponse::created("Postingan komunitas berhasil diterbitkan (+25 Club XP).", data))
}
