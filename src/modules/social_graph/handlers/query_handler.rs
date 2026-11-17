use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::{
    core::{
        extractors::{AuthUser, OptionalAuthUser},
        AppState,
    },
    modules::social_graph::{
        dto::{
            BatchRelationshipsRequest, BatchRelationshipsResponseDto, FollowRequestItemDto,
            PrivacySettingResponseDto, RelationshipSnapshotDto, SocialGraphCursorPageDto,
            SocialGraphListQuery, SocialGraphUserItemDto,
        },
        error::SocialGraphError,
        services::graph_list_service,
    },
};

pub async fn get_relationship_snapshot(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(target): Path<String>,
) -> Result<Json<RelationshipSnapshotDto>, SocialGraphError> {
    let viewer_id = maybe_claims.as_ref().map(|c| c.sub);
    let dto = state
        .social_graph_query
        .get_relationship_snapshot(&state.pool, viewer_id, &target)
        .await?;
    Ok(Json(dto))
}

pub async fn post_batch_relationships(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<BatchRelationshipsRequest>,
) -> Result<Json<BatchRelationshipsResponseDto>, SocialGraphError> {
    let dto = state
        .social_graph_query
        .batch_check_relationships(&state.pool, claims.sub, body.target_ids)
        .await?;
    Ok(Json(dto))
}

pub async fn get_followers_list(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(target): Path<String>,
    Query(q): Query<SocialGraphListQuery>,
) -> Result<Json<SocialGraphCursorPageDto<SocialGraphUserItemDto>>, SocialGraphError> {
    let viewer_id = maybe_claims.as_ref().map(|c| c.sub);
    let page = graph_list_service::list_connections(
        &state.social_graph_query,
        &state.pool,
        viewer_id,
        &target,
        "followers",
        q.cursor,
        q.limit,
    )
    .await?;
    Ok(Json(page))
}

pub async fn get_following_list(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(target): Path<String>,
    Query(q): Query<SocialGraphListQuery>,
) -> Result<Json<SocialGraphCursorPageDto<SocialGraphUserItemDto>>, SocialGraphError> {
    let viewer_id = maybe_claims.as_ref().map(|c| c.sub);
    let page = graph_list_service::list_connections(
        &state.social_graph_query,
        &state.pool,
        viewer_id,
        &target,
        "following",
        q.cursor,
        q.limit,
    )
    .await?;
    Ok(Json(page))
}

pub async fn get_mutual_connections_list(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(target): Path<String>,
    Query(q): Query<SocialGraphListQuery>,
) -> Result<Json<SocialGraphCursorPageDto<SocialGraphUserItemDto>>, SocialGraphError> {
    let viewer_id = maybe_claims.as_ref().map(|c| c.sub);
    let page = graph_list_service::list_connections(
        &state.social_graph_query,
        &state.pool,
        viewer_id,
        &target,
        "mutual",
        q.cursor,
        q.limit,
    )
    .await?;
    Ok(Json(page))
}

pub async fn get_pending_requests(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(q): Query<SocialGraphListQuery>,
) -> Result<Json<SocialGraphCursorPageDto<FollowRequestItemDto>>, SocialGraphError> {
    let page =
        graph_list_service::list_pending_requests(&state.pool, claims.sub, q.cursor, q.limit).await?;
    Ok(Json(page))
}

pub async fn get_privacy_setting(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<PrivacySettingResponseDto>, SocialGraphError> {
    let dto = graph_list_service::get_my_privacy_setting(&state.pool, claims.sub).await?;
    Ok(Json(dto))
}
