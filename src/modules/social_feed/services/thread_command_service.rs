use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::social_feed::{
    dto::{LikeToggleResponseDto, RepostToggleResponseDto, TryRecipeResponseDto},
    error::SocialFeedError,
    repositories::{interaction_repo, profile_projection_repo, thread_write_repo},
    services::profile_cqrs_service::ProfileCqrsService,
};

const XP_TRY_RECIPE_REWARD: i64 = 15;

pub async fn toggle_like(
    pg: &PgPool,
    thread_id: Uuid,
    user_id: Uuid,
) -> Result<LikeToggleResponseDto, SocialFeedError> {
    let (is_liked, likes_count) = interaction_repo::toggle_like_atomic(pg, thread_id, user_id).await?;
    Ok(LikeToggleResponseDto {
        thread_id,
        is_liked_by_me: is_liked,
        likes_count,
    })
}

pub async fn toggle_repost(
    pg: &PgPool,
    thread_id: Uuid,
    user_id: Uuid,
) -> Result<RepostToggleResponseDto, SocialFeedError> {
    let (is_reposted, reposts_count) =
        interaction_repo::toggle_repost_atomic(pg, thread_id, user_id).await?;
    Ok(RepostToggleResponseDto {
        thread_id,
        is_reposted_by_me: is_reposted,
        reposts_count,
    })
}

pub async fn record_recipe_try(
    pg: &PgPool,
    cqrs: &ProfileCqrsService,
    thread_id: Uuid,
    viewer_id: Option<Uuid>,
) -> Result<TryRecipeResponseDto, SocialFeedError> {
    let (author_id, new_count) = interaction_repo::increment_try_recipe(pg, thread_id).await?;
    let reward = if viewer_id == Some(author_id) {
        0
    } else {
        XP_TRY_RECIPE_REWARD
    };
    let (new_xp, new_level) =
        profile_projection_repo::award_xp_on_projection(pg, author_id, reward).await?;
    cqrs.invalidate_profile(author_id).await;

    Ok(TryRecipeResponseDto {
        thread_id,
        try_recipe_count: new_count,
        author_awarded_xp: reward,
        author_new_total_xp: new_xp,
        author_new_level: new_level,
    })
}

pub async fn delete_thread(
    pg: &PgPool,
    cqrs: &ProfileCqrsService,
    thread_id: Uuid,
    actor_id: Uuid,
) -> Result<(), SocialFeedError> {
    thread_write_repo::soft_delete_thread(pg, thread_id, actor_id).await?;
    cqrs.invalidate_profile(actor_id).await;
    Ok(())
}
