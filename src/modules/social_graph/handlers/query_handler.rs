use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

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

fn parse_uuid(sub: &str) -> Result<Uuid, SocialGraphError> {
    Uuid::parse_str(sub)
        .map_err(|_| SocialGraphError::Unauthorized("Identitas JWT tidak valid.".to_string()))
}

pub async fn get_relationship_snapshot(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(target): Path<String>,
) -> Result<Json<RelationshipSnapshotDto>, SocialGraphError> {
    let viewer_id = maybe_claims.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
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
    let viewer_id = parse_uuid(&claims.sub)?;
    let dto = state
        .social_graph_query
        .batch_check_relationships(&state.pool, viewer_id, body.target_ids)
        .await?;
    Ok(Json(dto))
}

pub async fn get_followers_list(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_claims): OptionalAuthUser,
    Path(target): Path<String>,
    Query(q): Query<SocialGraphListQuery>,
) -> Result<Json<SocialGraphCursorPageDto<SocialGraphUserItemDto>>, SocialGraphError> {
    let viewer_id = maybe_claims.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
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
    let viewer_id = maybe_claims.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
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
    let viewer_id = maybe_claims.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
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
    let owner_id = parse_uuid(&claims.sub)?;
    let page =
        graph_list_service::list_pending_requests(&state.pool, owner_id, q.cursor, q.limit).await?;
    Ok(Json(page))
}

pub async fn get_privacy_setting(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<PrivacySettingResponseDto>, SocialGraphError> {
    let owner_id = parse_uuid(&claims.sub)?;
    let dto = graph_list_service::get_my_privacy_setting(&state.pool, owner_id).await?;
    Ok(Json(dto))
}
