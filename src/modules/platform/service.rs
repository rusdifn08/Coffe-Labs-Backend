use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, security::Claims},
    modules::platform::{
        dto::{
            CreateFeedbackRequest, FeedbackQuery, UpdateFeedbackStatusRequest, VersionCheckQuery,
            VersionCheckResponse,
        },
        error::PlatformError,
        models::{AppVersionConfig, Feedback, FeedbackStats, UpdateAction},
        repository,
    },
};

pub fn evaluate_version_check(
    config: &AppVersionConfig,
    params: &VersionCheckQuery,
) -> VersionCheckResponse {
    let client_build = params.build_number.unwrap_or(1);
    let client_patch = params.patch_number.unwrap_or(0);

    let action = if config.is_force_update || client_build < config.min_build_number {
        UpdateAction::ForceStoreUpdate
    } else if client_build < config.latest_build_number {
        UpdateAction::FlexibleStoreUpdate
    } else if client_build == config.latest_build_number {
        if client_patch < config.latest_patch_number {
            if config.is_patch_required {
                UpdateAction::ForceOtaPatch
            } else {
                UpdateAction::SilentOtaPatch
            }
        } else {
            UpdateAction::None
        }
    } else {
        UpdateAction::None
    };

    let action_code = match action {
        UpdateAction::None => "UP_TO_DATE",
        UpdateAction::SilentOtaPatch => "SILENT_OTA_PATCH",
        UpdateAction::ForceOtaPatch => "FORCE_OTA_PATCH",
        UpdateAction::FlexibleStoreUpdate => "FLEXIBLE_STORE_UPDATE",
        UpdateAction::ForceStoreUpdate => "FORCE_STORE_UPDATE",
    }
    .to_string();

    let target_patch = if config.latest_patch_number > 0 {
        Some(config.latest_patch_number)
    } else {
        None
    };

    VersionCheckResponse {
        action,
        action_code,
        target_patch,
        latest_base_version: config.latest_base_version.clone(),
        latest_build_number: config.latest_build_number,
        is_force_update: config.is_force_update,
        release_notes: config.release_notes.clone(),
        download_url: config.download_url.clone(),
        checksum_sha256: config.checksum_sha256.clone(),
    }
}

pub async fn create_feedback(
    pool: &PgPool,
    claims_opt: Option<Claims>,
    req: CreateFeedbackRequest,
) -> AppResult<Feedback> {
    let (user_id, username, user_role) = match claims_opt {
        Some(c) => (Uuid::parse_str(&c.sub).ok(), c.username, c.role),
        None => (
            None,
            req.guest_name.unwrap_or_else(|| "Tamu / Trial".to_string()),
            "guest".to_string(),
        ),
    };

    let rating = req.rating.unwrap_or(5).clamp(1, 5);
    let app_version = req.app_version.unwrap_or_else(|| "1.0.0".to_string());
    let device_info = req.device_info.unwrap_or_else(|| "Mobile".to_string());

    repository::insert_feedback(
        pool,
        user_id,
        &username,
        &user_role,
        &req.category.trim().to_lowercase(),
        req.feature_affected.trim(),
        req.title.trim(),
        req.description.trim(),
        rating,
        &app_version,
        &device_info,
    )
    .await
}

pub async fn list_admin_feedbacks(pool: &PgPool, q: FeedbackQuery) -> AppResult<Vec<Feedback>> {
    repository::list_feedbacks(pool, q.status.as_deref(), q.category.as_deref()).await
}

pub async fn update_feedback(
    pool: &PgPool,
    id: Uuid,
    req: UpdateFeedbackStatusRequest,
) -> AppResult<Feedback> {
    repository::update_feedback_status(pool, id, req.status.trim(), req.admin_notes.as_deref())
        .await?
        .ok_or_else(|| PlatformError::FeedbackNotFound.into())
}

pub async fn remove_feedback(pool: &PgPool, id: Uuid) -> AppResult<()> {
    let affected = repository::delete_feedback(pool, id).await?;
    if affected == 0 {
        return Err(PlatformError::FeedbackNotFound.into());
    }
    Ok(())
}

pub async fn fetch_stats(pool: &PgPool) -> AppResult<FeedbackStats> {
    repository::get_feedback_stats(pool).await
}
