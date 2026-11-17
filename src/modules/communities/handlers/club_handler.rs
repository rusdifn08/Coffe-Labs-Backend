use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::communities::{
        dto::{CreateCommunityRequest, DiscoverCommunitiesQuery},
        services::{club_discovery_service, club_workspace_service, member_invite_service},
    },
};

pub async fn discover_communities(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<DiscoverCommunitiesQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().map(|c| c.sub);
    let data = club_discovery_service::discover_communities(&state.pool, viewer_id, query).await?;
    Ok(ApiResponse::ok("Daftar Coffee Club berhasil dimuat.", data))
}

pub async fn create_community(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateCommunityRequest>,
) -> AppResult<impl IntoResponse> {
    let invited_ids = req.invited_user_ids.clone();
    let invite_msg = req.invite_message.clone();
    let data = club_workspace_service::create_community(
        &state.pool,
        state.secret_join_service.hmac_key(),
        &state.club_cache,
        claims.sub,
        req,
    )
    .await?;

    if !invited_ids.is_empty() {
        let _ = member_invite_service::send_direct_invitations(
            &state.pool,
            &state.club_cache,
            &state.notifications,
            claims.sub,
            data.community.id,
            &invited_ids,
            Some("BARISTA_MEMBER"),
            invite_msg.as_deref(),
        )
        .await;
    }

    Ok(ApiResponse::created("Coffee Club berhasil didirikan.", data))
}

pub async fn get_workspace(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().map(|c| c.sub);
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
    let data = club_workspace_service::join_public_club(
        &state.pool,
        &state.club_cache,
        claims.sub,
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
    let viewer_id = auth.as_ref().map(|c| c.sub);
    let data = club_discovery_service::get_leaderboard(
        &state.pool,
        &state.club_cache,
        viewer_id,
        community_id,
    )
    .await?;
    Ok(ApiResponse::ok("Leaderboard Barista Club berhasil dimuat.", data))
}
