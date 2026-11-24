use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::platform::models::{Feedback, FeedbackStats},
};

pub async fn insert_feedback(
    pool: &PgPool,
    user_id: Option<Uuid>,
    username: &str,
    user_role: &str,
    category: &str,
    feature_affected: &str,
    title: &str,
    description: &str,
    rating: i32,
    app_version: &str,
    device_info: &str,
) -> AppResult<Feedback> {
    let fb = sqlx::query_as::<_, Feedback>(
        r#"INSERT INTO feedbacks (
            user_id, username, user_role, category, feature_affected,
            title, description, rating, app_version, device_info, status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending')
        RETURNING *"#,
    )
    .bind(user_id)
    .bind(username)
    .bind(user_role)
    .bind(category)
    .bind(feature_affected)
    .bind(title)
    .bind(description)
    .bind(rating)
    .bind(app_version)
    .bind(device_info)
    .fetch_one(pool)
    .await?;
    Ok(fb)
}

pub async fn list_feedbacks(
    pool: &PgPool,
    status: Option<&str>,
    category: Option<&str>,
) -> AppResult<Vec<Feedback>> {
    let status_filter = status.filter(|s| !s.trim().is_empty() && *s != "all");
    let cat_filter = category.filter(|c| !c.trim().is_empty() && *c != "all");

    let rows = sqlx::query_as::<_, Feedback>(
        r#"SELECT * FROM feedbacks
           WHERE ($1::text IS NULL OR status = $1)
             AND ($2::text IS NULL OR category = $2)
           ORDER BY created_at DESC"#,
    )
    .bind(status_filter)
    .bind(cat_filter)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn update_feedback_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    admin_notes: Option<&str>,
) -> AppResult<Option<Feedback>> {
    let row = sqlx::query_as::<_, Feedback>(
        r#"UPDATE feedbacks
           SET status = $1, admin_notes = COALESCE($2, admin_notes), updated_at = NOW()
           WHERE id = $3
           RETURNING *"#,
    )
    .bind(status)
    .bind(admin_notes)
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn delete_feedback(pool: &PgPool, id: Uuid) -> AppResult<u64> {
    let res = sqlx::query("DELETE FROM feedbacks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn get_feedback_stats(pool: &PgPool) -> AppResult<FeedbackStats> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks").fetch_one(pool).await?;
    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'pending'").fetch_one(pool).await?;
    let in_progress: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'in_progress'").fetch_one(pool).await?;
    let resolved: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE status = 'resolved'").fetch_one(pool).await?;
    let bug_reports: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE category = 'bug_report'").fetch_one(pool).await?;
    let feature_requests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM feedbacks WHERE category = 'feature_request'").fetch_one(pool).await?;
    let avg: f64 = sqlx::query_scalar("SELECT COALESCE(AVG(rating)::FLOAT8, 5.0) FROM feedbacks WHERE rating IS NOT NULL").fetch_one(pool).await?;

    Ok(FeedbackStats {
        total,
        pending,
        in_progress,
        resolved,
        bug_reports,
        feature_requests,
        average_rating: (avg * 10.0).round() / 10.0,
    })
}
