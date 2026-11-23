use axum::{extract::State, response::IntoResponse};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{OptionalAuthUser, ValidatedJson},
        AppState,
    },
    modules::social::{dto::CreateSocialPostRequest, services::post_service},
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
