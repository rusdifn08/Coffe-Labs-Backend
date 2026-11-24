use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, jwt::JwtClaims},
    modules::platform::{
        dto::{
            CreateFeedbackRequest, FeedbackQuery, UpdateFeedbackStatusRequest, VersionCheckQuery,
            VersionCheckResponse,
        },
        error::PlatformError,
        models::{AppVersionConfig, Feedback, FeedbackStats},
        repository, version_evaluator,
    },
};

pub fn evaluate_version_check(
    config: &AppVersionConfig,
    params: &VersionCheckQuery,
) -> VersionCheckResponse {
    version_evaluator::evaluate_version_check(config, params)
}

pub async fn create_feedback(
    pool: &PgPool,
    claims_opt: Option<JwtClaims>,
    req: CreateFeedbackRequest,
) -> AppResult<Feedback> {
    let (user_id, username, user_role) = match claims_opt {
        Some(c) => (Some(c.sub), c.username, c.role),
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
