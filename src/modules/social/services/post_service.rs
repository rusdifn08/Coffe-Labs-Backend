use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, security::Claims},
    modules::social::{
        dto::{
            CopyRecipeResponse, CreateCommentRequest, CreateSocialPostRequest, FollowStatsResponse,
            FollowToggleRequest, LikePostResponse, SocialPostResponse,
        },
        error::SocialError,
        models::SocialComment,
        repositories::post_repo,
        services::feed_service::map_post_to_response,
    },
};

pub async fn create_post(
    pool: &PgPool,
    auth: Option<Claims>,
    req: CreateSocialPostRequest,
) -> AppResult<SocialPostResponse> {
    let user_id = auth.as_ref().and_then(|u| Uuid::parse_str(&u.sub).ok());
    let author_name = if let Some(ref u) = auth {
        u.username.clone()
    } else if let Some(ref name) = req.author_name {
        if !name.trim().is_empty() {
            name.trim().to_string()
        } else {
            "Barista Komunitas".to_string()
        }
    } else {
        "Barista Komunitas".to_string()
    };

    let avatar = req
        .author_avatar
        .unwrap_or_else(|| "assets/images/characters/avatar_1.png".to_string());
    let role = req.author_role.unwrap_or_else(|| "Barista".to_string());
    let tags_json = serde_json::to_value(req.tags.unwrap_or_default()).unwrap_or_else(|_| json!([]));

    let post = post_repo::insert_post(
        pool,
        user_id,
        &author_name,
        Some(&avatar),
        &role,
        req.content.trim(),
        req.recipe_id,
        req.recipe_snapshot,
        tags_json,
    )
    .await?;

    Ok(map_post_to_response(post, false, None))
}

pub async fn toggle_like(
    pool: &PgPool,
    post_id: Uuid,
    username: &str,
) -> AppResult<LikePostResponse> {
    let clean_user = username.trim().to_lowercase();
    let (is_liked, likes_count) = post_repo::toggle_post_like(pool, post_id, &clean_user).await?;
    Ok(LikePostResponse {
        is_liked,
        likes_count,
    })
}

pub async fn get_comments(pool: &PgPool, post_id: Uuid) -> AppResult<Vec<SocialComment>> {
    post_repo::list_comments(pool, post_id).await
}

pub async fn add_comment(
    pool: &PgPool,
    post_id: Uuid,
    auth: Option<Claims>,
    req: CreateCommentRequest,
) -> AppResult<SocialComment> {
    let user_id = auth.as_ref().and_then(|u| Uuid::parse_str(&u.sub).ok());
    let author_name = if let Some(ref u) = auth {
        u.username.clone()
    } else if let Some(ref name) = req.author_name {
        if !name.trim().is_empty() {
            name.trim().to_string()
        } else {
            "Barista".to_string()
        }
    } else {
        "Barista".to_string()
    };

    let avatar = req
        .author_avatar
        .unwrap_or_else(|| "assets/images/characters/avatar_1.png".to_string());

    post_repo::insert_comment(
        pool,
        post_id,
        req.parent_id,
        user_id,
        &author_name,
        Some(&avatar),
        req.content.trim(),
    )
    .await
}

pub async fn copy_recipe(pool: &PgPool, post_id: Uuid) -> AppResult<CopyRecipeResponse> {
    let post = post_repo::increment_recipe_copy(pool, post_id)
        .await?
        .ok_or(SocialError::PostNotFound)?;

    Ok(CopyRecipeResponse {
        copies_count: post.copies_count,
        recipe_snapshot: post.recipe_snapshot,
    })
}

pub async fn toggle_follow_user(
    pool: &PgPool,
    req: FollowToggleRequest,
) -> AppResult<FollowStatsResponse> {
    let follower = req.follower_username.trim().to_lowercase();
    let following = req.following_username.trim().to_lowercase();
    if follower == following {
        return Err(SocialError::CannotFollowSelf.into());
    }
    let is_following = post_repo::toggle_follow(pool, &follower, &following).await?;
    let (followers_count, following_count, _) =
        post_repo::get_follow_stats(pool, &following, Some(&follower)).await?;

    Ok(FollowStatsResponse {
        username: following,
        followers_count,
        following_count,
        is_following,
    })
}
