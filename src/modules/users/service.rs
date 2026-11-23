use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{
        email_service::normalize_email,
        error::{AppError, AppResult},
    },
    modules::{
        auth::{models::UserSummary, password_reset_repo},
        chat::services::moderation_normalizer::contains_multilingual_profanity,
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

pub async fn update_user_tier(
    pool: &PgPool,
    user_id: Uuid,
    tier: &str,
    trial_days: Option<i32>,
) -> AppResult<UserSummary> {
    let res = repository::update_user_tier(pool, user_id, tier, trial_days)
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
    let clean_name = req
        .full_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(name) = clean_name {
        if contains_multilingual_profanity(name) {
            return Err(AppError::Validation(
                "Nama profil mengandung kata kasar atau tidak pantas.".to_string(),
            ));
        }
    }
    if let Some(bio) = req.bio.as_deref() {
        if contains_multilingual_profanity(bio) {
            return Err(AppError::Validation(
                "Bio profil mengandung kata kasar atau tidak pantas.".to_string(),
            ));
        }
    }
    let clean_email = match req.email.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => {
            let valid = normalize_email(raw).ok_or_else(|| {
                AppError::Validation("Format alamat email tidak valid. Contoh: nama@gmail.com".to_string())
            })?;
            if password_reset_repo::exists_by_email(pool, &valid, Some(user_id)).await? {
                return Err(AppError::Conflict(format!(
                    "Email '{}' sudah digunakan oleh akun lain.",
                    valid
                )));
            }
            Some(valid)
        }
        None => None,
    };
    let clean_avatar = req
        .avatar_url
        .as_deref()
        .map(crate::core::storage::auto_optimize_url)
        .filter(|s| !s.is_empty());

    let updated = repository::update_profile(
        pool,
        user_id,
        clean_name,
        clean_email.as_deref(),
        clean_avatar.as_deref(),
        req.bio.as_deref(),
        req.equipment_json,
    )
    .await?
    .ok_or(UsersError::UserNotFoundOrProtected)?;

    // Sinkronkan juga ke user_profile_projections agar Social Coffee & pencarian langsung terbarui.
    let _ = sqlx::query(
        r#"UPDATE user_profile_projections
           SET display_name = $2,
               avatar_url = COALESCE($3, avatar_url),
               bio = COALESCE($4, bio),
               updated_at = NOW()
           WHERE user_id = $1"#,
    )
    .bind(user_id)
    .bind(&updated.full_name)
    .bind(&updated.avatar_url)
    .bind(&updated.bio)
    .execute(pool)
    .await;

    crate::modules::auth::repository::invalidate_user_cache_all();
    Ok(updated)
}
