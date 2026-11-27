use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        error::{AppError, AppResult},
        jwt::JwtClaims,
    },
    modules::{
        chat::services::moderation_guard::{ModerationGuard, ModerationMode},
        social::{
            dto::{
                CopyRecipeResponse, CreateCommentRequest, CreateSocialPostRequest,
                FollowStatsResponse, FollowToggleRequest, LikePostResponse, ReportPostRequest,
                ReportPostResponse, RepostPostResponse, SavePostResponse, SocialPostResponse,
                UpdateSocialPostRequest,
            },
            error::SocialError,
            models::SocialComment,
            repositories::post_repo,
            services::feed_service::map_post_to_response,
        },
    },
};

fn enforce_clean_content(guard: &ModerationGuard, raw: &str) -> AppResult<()> {
    guard
        .check_text(raw, ModerationMode::RejectMessage)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    Ok(())
}

pub async fn create_post(
    pool: &PgPool,
    moderation: &ModerationGuard,
    auth: Option<JwtClaims>,
    req: CreateSocialPostRequest,
) -> AppResult<SocialPostResponse> {
    enforce_clean_content(moderation, &req.content)?;
    if let Some(ref tags) = req.tags {
        for tag in tags {
            enforce_clean_content(moderation, tag)?;
        }
    }
    let user_id = auth.as_ref().map(|u| u.sub);
    let author_name = if let Some(ref u) = auth {
        u.username.clone()
    } else if let Some(ref name) = req.author_name {
        if !name.trim().is_empty() {
            enforce_clean_content(moderation, name)?;
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

    Ok(map_post_to_response(post, false, false, false, None))
}

fn is_post_owner(post_user_id: Option<Uuid>, post_author: &str, auth: &Option<JwtClaims>, req_author: Option<&str>) -> bool {
    if let Some(u) = auth {
        if let Some(uid) = post_user_id {
            if uid == u.sub {
                return true;
            }
        }
        if post_author.eq_ignore_ascii_case(u.username.trim()) {
            return true;
        }
        if u.role.eq_ignore_ascii_case("admin") {
            return true;
        }
    }
    if let Some(name) = req_author {
        if !name.trim().is_empty() && post_author.eq_ignore_ascii_case(name.trim()) {
            return true;
        }
    }
    false
}

pub async fn update_post(
    pool: &PgPool,
    moderation: &ModerationGuard,
    post_id: Uuid,
    auth: Option<JwtClaims>,
    req: UpdateSocialPostRequest,
) -> AppResult<SocialPostResponse> {
    enforce_clean_content(moderation, &req.content)?;
    if let Some(ref tags) = req.tags {
        for tag in tags {
            enforce_clean_content(moderation, tag)?;
        }
    }

    let existing = post_repo::find_post_by_id(pool, post_id)
        .await?
        .ok_or(SocialError::PostNotFound)?;

    if !is_post_owner(existing.user_id, &existing.author_name, &auth, req.author_name.as_deref()) {
        return Err(SocialError::ForbiddenOwner.into());
    }

    let tags_json = match req.tags {
        Some(t) => serde_json::to_value(t).unwrap_or_else(|_| json!([])),
        None => existing.tags.unwrap_or_else(|| json!([])),
    };
    let recipe_id = req.recipe_id.or(existing.recipe_id);
    let recipe_snapshot = req.recipe_snapshot.or(existing.recipe_snapshot);

    let updated = post_repo::update_post(
        pool,
        post_id,
        req.content.trim(),
        recipe_id,
        recipe_snapshot,
        tags_json,
    )
    .await?
    .ok_or(SocialError::PostNotFound)?;

    Ok(map_post_to_response(updated, false, false, false, None))
}

pub async fn delete_post(
    pool: &PgPool,
    post_id: Uuid,
    auth: Option<JwtClaims>,
    req_username: Option<&str>,
) -> AppResult<()> {
    let existing = post_repo::find_post_by_id(pool, post_id)
        .await?
        .ok_or(SocialError::PostNotFound)?;

    if !is_post_owner(existing.user_id, &existing.author_name, &auth, req_username) {
        return Err(SocialError::ForbiddenOwner.into());
    }

    post_repo::delete_post(pool, post_id).await?;
    Ok(())
}

pub async fn toggle_repost(
    pool: &PgPool,
    post_id: Uuid,
    username: &str,
) -> AppResult<RepostPostResponse> {
    let clean_user = username.trim().to_lowercase();
    let (is_reposted, reposts_count) =
        post_repo::toggle_post_repost(pool, post_id, &clean_user).await?;
    Ok(RepostPostResponse {
        is_reposted,
        reposts_count,
    })
}

pub async fn toggle_save(
    pool: &PgPool,
    post_id: Uuid,
    username: &str,
) -> AppResult<SavePostResponse> {
    let clean_user = username.trim().to_lowercase();
    let is_saved = post_repo::toggle_post_save(pool, post_id, &clean_user).await?;
    Ok(SavePostResponse { is_saved })
}

pub async fn report_post(
    pool: &PgPool,
    post_id: Uuid,
    req: ReportPostRequest,
) -> AppResult<ReportPostResponse> {
    let clean_user = req.username.trim().to_lowercase();
    post_repo::insert_post_report(pool, post_id, &clean_user, req.reason.trim()).await?;
    Ok(ReportPostResponse {
        reported: true,
        post_id,
    })
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
    moderation: &ModerationGuard,
    post_id: Uuid,
    auth: Option<JwtClaims>,
    req: CreateCommentRequest,
) -> AppResult<SocialComment> {
    enforce_clean_content(moderation, &req.content)?;
    let user_id = auth.as_ref().map(|u| u.sub);
    let author_name = if let Some(ref u) = auth {
        u.username.clone()
    } else if let Some(ref name) = req.author_name {
        if !name.trim().is_empty() {
            enforce_clean_content(moderation, name)?;
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
