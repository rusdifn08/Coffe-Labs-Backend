use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use sqlx::PgPool;

use crate::{
    core::error::{ApiResponse, AppResult},
    modules::social::{dto::SocialFeedQuery, services::feed_service},
};

pub async fn get_social_posts(
    Query(query): Query<SocialFeedQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let posts = feed_service::get_feed_list(&pool, query).await?;
    Ok(ApiResponse::ok("Feed Social Coffee berhasil dimuat", posts))
}

pub async fn get_social_posts_cursor(
    Query(query): Query<SocialFeedQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let page = feed_service::get_cursor_feed(&pool, query).await?;
    Ok(ApiResponse::ok("Cursor Feed Social Coffee berhasil dimuat", page))
}
