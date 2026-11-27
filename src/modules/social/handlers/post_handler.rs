use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::social::{
        dto::{CreateSocialPostRequest, DeleteSocialPostQuery, UpdateSocialPostRequest},
        services::post_service,
    },
};

pub async fn create_social_post(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateSocialPostRequest>,
) -> AppResult<impl IntoResponse> {
    let post =
        post_service::create_post(&state.pool, &state.chat_moderation, auth, payload).await?;
    Ok(ApiResponse::created(
        "Postingan berhasil dipublikasikan ke Social Coffee",
        post,
    ))
}

pub async fn update_social_post(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<UpdateSocialPostRequest>,
) -> AppResult<impl IntoResponse> {
    let post =
        post_service::update_post(&state.pool, &state.chat_moderation, id, auth, payload).await?;
    Ok(ApiResponse::ok("Postingan berhasil diperbarui", post))
}

pub async fn delete_social_post(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<DeleteSocialPostQuery>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    post_service::delete_post(&state.pool, id, auth, query.username.as_deref()).await?;
    Ok(ApiResponse::ok(
        "Postingan berhasil dihapus",
        json!({ "deleted": true, "id": id }),
    ))
}
