use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::communities::{
        dto::{CreateCommunityRequest, DiscoverCommunitiesQuery},
        services::{club_discovery_service, club_workspace_service},
    },
};

fn parse_uid(raw: &str) -> AppResult<Uuid> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest("User ID tidak valid.".into()))
}

pub async fn discover_communities(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<DiscoverCommunitiesQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = club_discovery_service::discover_communities(&state.pool, viewer_id, query).await?;
    Ok(ApiResponse::ok("Daftar Coffee Club berhasil dimuat.", data))
}

pub async fn create_community(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateCommunityRequest>,
) -> AppResult<impl IntoResponse> {
    let owner_id = parse_uid(&claims.sub)?;
    let data = club_workspace_service::create_community(
        &state.pool,
        state.secret_join_service.hmac_key(),
        &state.club_cache,
        owner_id,
        req,
    )
    .await?;
    Ok(ApiResponse::created("Coffee Club berhasil didirikan.", data))
}

pub async fn get_workspace(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = club_workspace_service::get_workspace(
        &state.pool,
        &state.club_cache,
        viewer_id,
        community_id,
    )
    .await?;
    Ok(ApiResponse::ok("Workspace Coffee Club berhasil dimuat.", data))
}

pub async fn join_public_club(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let data = club_workspace_service::join_public_club(
        &state.pool,
        &state.club_cache,
        user_id,
        community_id,
    )
    .await?;
    Ok(ApiResponse::ok("Berhasil bergabung dengan Coffee Club.", data))
}

pub async fn get_leaderboard(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let data = club_discovery_service::get_leaderboard(
        &state.pool,
        &state.club_cache,
        viewer_id,
        community_id,
    )
    .await?;
    Ok(ApiResponse::ok("Leaderboard Barista Club berhasil dimuat.", data))
}
