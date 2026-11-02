use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{OptionalAuthUser, ValidatedJson},
    },
    modules::social::{
        dto::{CreateCommentRequest, FollowStatsResponse, FollowToggleRequest, LikePostRequest},
        repositories::post_repo,
        services::post_service,
    },
};

pub async fn toggle_like_social_post(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
    ValidatedJson(payload): ValidatedJson<LikePostRequest>,
) -> AppResult<impl IntoResponse> {
    let res = post_service::toggle_like(&pool, id, &payload.username).await?;
    let msg = if res.is_liked {
        "Postingan disukai"
    } else {
        "Batal menyukai postingan"
    };
    Ok(ApiResponse::ok(msg, res))
}

pub async fn get_social_post_comments(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let comments = post_service::get_comments(&pool, id).await?;
    Ok(ApiResponse::ok("Daftar komentar berhasil dimuat", comments))
}

pub async fn add_social_post_comment(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
    ValidatedJson(payload): ValidatedJson<CreateCommentRequest>,
) -> AppResult<impl IntoResponse> {
    let comment = post_service::add_comment(&pool, id, auth, payload).await?;
    Ok(ApiResponse::created("Komentar berhasil dikirim", comment))
}

pub async fn copy_social_post_recipe(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let res = post_service::copy_recipe(&pool, id).await?;
    Ok(ApiResponse::ok("Resep berhasil disalin ke koleksi Anda!", res))
}

pub async fn toggle_follow(
    State(pool): State<PgPool>,
    ValidatedJson(payload): ValidatedJson<FollowToggleRequest>,
) -> AppResult<impl IntoResponse> {
    let stats = post_service::toggle_follow_user(&pool, payload).await?;
    Ok(ApiResponse::ok("Status mengikuti berhasil diperbarui", stats))
}

pub async fn get_follow_stats(
    Path(username): Path<String>,
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let clean = username.trim().to_lowercase();
    let viewer = auth.as_ref().map(|u| u.username.as_str());
    let (followers_count, following_count, is_following) =
        post_repo::get_follow_stats(&pool, &clean, viewer).await?;
    Ok(ApiResponse::ok(
        "Statistik komunitas berhasil dimuat",
        FollowStatsResponse {
            username: clean,
            followers_count,
            following_count,
            is_following,
        },
    ))
}
