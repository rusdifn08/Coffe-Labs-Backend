use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::OptionalAuthUser,
        AppState,
    },
    modules::social_feed::{
        dto::{ProfileFeedTab, ProfileTabFeedQuery, TimelineFeedQuery},
        services::timeline_query_service,
    },
};

pub async fn get_profile_tab_threads(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(target): Path<String>,
    Query(query): Query<ProfileTabFeedQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let tab = ProfileFeedTab::from_query(query.tab.as_deref());
    let page = timeline_query_service::get_profile_tab_feed(
        &state.pool,
        viewer_id,
        &target,
        tab,
        query.cursor.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await?;
    Ok(ApiResponse::ok("Feed aktivitas profil berhasil dimuat.", page))
}

pub async fn get_global_timeline(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<TimelineFeedQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let page = timeline_query_service::get_global_timeline(
        &state.pool,
        viewer_id,
        query.scope.as_deref(),
        query.cursor.as_deref(),
        query.limit.unwrap_or(20),
    )
    .await?;
    Ok(ApiResponse::ok("Linimasa Social Coffee berhasil dimuat.", page))
}

pub async fn get_thread_detail(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let viewer_id = auth.as_ref().and_then(|c| Uuid::parse_str(&c.sub).ok());
    let detail =
        timeline_query_service::get_thread_with_nested_replies(&state.pool, viewer_id, id).await?;
    Ok(ApiResponse::ok("Detail diskusi & balasan berhasil dimuat.", detail))
}
