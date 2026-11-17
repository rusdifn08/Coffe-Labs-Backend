use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    core::{extractors::AuthUser, AppState},
    modules::social_graph::{
        dto::{
            FollowMutationResponseDto, PendingRequestActionResponseDto, PrivacySettingResponseDto,
            UpdatePrivacyRequest,
        },
        error::SocialGraphError,
        services::graph_privacy_service,
    },
};

pub async fn post_follow(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(target): Path<String>,
) -> Result<Json<FollowMutationResponseDto>, SocialGraphError> {
    let dto = state
        .social_graph_cmd
        .follow_user(&state.pool, claims.sub, &target)
        .await?;
    Ok(Json(dto))
}

pub async fn delete_follow(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(target): Path<String>,
) -> Result<Json<FollowMutationResponseDto>, SocialGraphError> {
    let dto = state
        .social_graph_cmd
        .unfollow_user(&state.pool, claims.sub, &target)
        .await?;
    Ok(Json(dto))
}

pub async fn post_accept_request(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(requester_id): Path<Uuid>,
) -> Result<Json<PendingRequestActionResponseDto>, SocialGraphError> {
    let dto = graph_privacy_service::handle_follow_request(
        &state.social_graph_cmd,
        &state.pool,
        claims.sub,
        requester_id,
        true,
    )
    .await?;
    Ok(Json(dto))
}

pub async fn post_reject_request(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(requester_id): Path<Uuid>,
) -> Result<Json<PendingRequestActionResponseDto>, SocialGraphError> {
    let dto = graph_privacy_service::handle_follow_request(
        &state.social_graph_cmd,
        &state.pool,
        claims.sub,
        requester_id,
        false,
    )
    .await?;
    Ok(Json(dto))
}

pub async fn delete_follower(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(follower_id): Path<Uuid>,
) -> Result<Json<FollowMutationResponseDto>, SocialGraphError> {
    let dto = graph_privacy_service::remove_follower(
        &state.social_graph_cmd,
        &state.pool,
        claims.sub,
        follower_id,
    )
    .await?;
    Ok(Json(dto))
}

pub async fn patch_privacy_setting(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<UpdatePrivacyRequest>,
) -> Result<Json<PrivacySettingResponseDto>, SocialGraphError> {
    let dto = graph_privacy_service::update_privacy(
        &state.social_graph_cmd,
        &state.pool,
        claims.sub,
        body.is_private_account,
    )
    .await?;
    Ok(Json(dto))
}
