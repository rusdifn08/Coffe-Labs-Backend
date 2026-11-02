use axum::{extract::State, response::IntoResponse};
use sqlx::PgPool;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{OptionalAuthUser, ValidatedJson},
    },
    modules::social::{dto::CreateSocialPostRequest, services::post_service},
};

pub async fn create_social_post(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
    ValidatedJson(payload): ValidatedJson<CreateSocialPostRequest>,
) -> AppResult<impl IntoResponse> {
    let post = post_service::create_post(&pool, auth, payload).await?;
    Ok(ApiResponse::created(
        "Postingan berhasil dipublikasikan ke Social Coffee",
        post,
    ))
}
