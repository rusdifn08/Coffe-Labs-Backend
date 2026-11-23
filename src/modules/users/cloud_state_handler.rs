use axum::{extract::State, response::IntoResponse};
use sqlx::PgPool;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, ValidatedJson},
    },
    modules::users::{cloud_state_dto::UpsertUserCloudStateRequest, cloud_state_repo},
};

pub async fn get_my_cloud_state(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let state = cloud_state_repo::fetch_cloud_state(&pool, claims.sub).await?;
    Ok(ApiResponse::ok("Cloud state pengguna berhasil dimuat.", state))
}

pub async fn put_my_cloud_state(
    State(pool): State<PgPool>,
    AuthUser(claims): AuthUser,
    ValidatedJson(payload): ValidatedJson<UpsertUserCloudStateRequest>,
) -> AppResult<impl IntoResponse> {
    let state = cloud_state_repo::upsert_cloud_state(&pool, claims.sub, &payload).await?;
    Ok(ApiResponse::ok("Cloud state pengguna berhasil disinkronkan.", state))
}
