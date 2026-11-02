use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::{
        auth::models::UserSummary,
        users::{
            dto::UpdateProfileRequest,
            error::UsersError,
            models::{AdminUserStats, BaristaProfile},
            repository,
        },
    },
};

pub async fn get_all_users(pool: &PgPool) -> AppResult<Vec<UserSummary>> {
    repository::list_all_users(pool).await
}

pub async fn get_user_stats(pool: &PgPool) -> AppResult<AdminUserStats> {
    repository::fetch_user_stats(pool).await
}

pub async fn accept_user(pool: &PgPool, user_id: Uuid) -> AppResult<UserSummary> {
    let res = repository::update_user_status(pool, user_id, "approved", false)
        .await?
        .ok_or_else(|| UsersError::UserNotFoundOrProtected.into());
    crate::modules::auth::repository::invalidate_user_cache_all();
    res
}

pub async fn decline_user(pool: &PgPool, user_id: Uuid) -> AppResult<UserSummary> {
    let res = repository::update_user_status(pool, user_id, "declined", true)
        .await?
        .ok_or_else(|| UsersError::UserNotFoundOrProtected.into());
    crate::modules::auth::repository::invalidate_user_cache_all();
    res
}

pub async fn delete_user(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
    let affected = repository::delete_non_admin_user(pool, user_id).await?;
    if affected == 0 {
        return Err(UsersError::UserNotFoundOrProtected.into());
    }
    crate::modules::auth::repository::invalidate_user_cache_all();
    Ok(())
}

pub async fn get_barista_profile(pool: &PgPool, user_id: Uuid) -> AppResult<BaristaProfile> {
    repository::get_profile(pool, user_id)
        .await?
        .ok_or_else(|| UsersError::UserNotFoundOrProtected.into())
}

pub async fn update_barista_profile(
    pool: &PgPool,
    user_id: Uuid,
    req: UpdateProfileRequest,
) -> AppResult<BaristaProfile> {
    repository::update_profile(
        pool,
        user_id,
        req.full_name.as_deref(),
        req.avatar_url.as_deref(),
        req.bio.as_deref(),
        req.equipment_json,
    )
    .await?
    .ok_or_else(|| UsersError::UserNotFoundOrProtected.into())
}
