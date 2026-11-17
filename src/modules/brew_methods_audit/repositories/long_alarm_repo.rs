use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::brew_methods_audit::{
    dto::ScheduleLongAlarmRequest,
    error::BrewAuditError,
    models::LongBrewAlarmRow,
};

pub async fn insert_alarm(
    pool: &PgPool,
    user_id: Uuid,
    req: &ScheduleLongAlarmRequest,
    checkpoints: serde_json::Value,
    target_finish_at: DateTime<Utc>,
) -> Result<LongBrewAlarmRow, BrewAuditError> {
    let slug = req
        .method_slug
        .clone()
        .unwrap_or_else(|| "cold_brew".to_string());
    let temp = req.ambient_temp_c.unwrap_or(4.0);
    let row = sqlx::query_as::<_, LongBrewAlarmRow>(
        r#"
        INSERT INTO long_brew_scheduled_alarms (
            user_id, recipe_id, recipe_title, method_slug, brew_mode,
            coffee_dose_g, water_ml, ambient_temp_c, drip_rate_bpm,
            started_at, target_finish_at, total_duration_sec, checkpoints_json, alarm_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), $10, $11, $12, 'ACTIVE')
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(req.recipe_id)
    .bind(req.recipe_title.trim())
    .bind(slug)
    .bind(req.brew_mode.trim().to_ascii_uppercase())
    .bind(req.coffee_dose_g)
    .bind(req.water_ml)
    .bind(temp)
    .bind(req.drip_rate_bpm)
    .bind(target_finish_at)
    .bind(req.duration_seconds)
    .bind(checkpoints)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn list_user_alarms(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<LongBrewAlarmRow>, BrewAuditError> {
    let rows = sqlx::query_as::<_, LongBrewAlarmRow>(
        r#"
        SELECT *
        FROM long_brew_scheduled_alarms
        WHERE user_id = $1 AND alarm_status IN ('ACTIVE', 'SNOOZED', 'RINGING')
        ORDER BY target_finish_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn snooze_alarm(
    pool: &PgPool,
    alarm_id: Uuid,
    user_id: Uuid,
    new_finish_at: DateTime<Utc>,
) -> Result<LongBrewAlarmRow, BrewAuditError> {
    let maybe = sqlx::query_as::<_, LongBrewAlarmRow>(
        r#"
        UPDATE long_brew_scheduled_alarms
        SET
            alarm_status = 'SNOOZED',
            target_finish_at = $3,
            snooze_count = snooze_count + 1,
            updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(alarm_id)
    .bind(user_id)
    .bind(new_finish_at)
    .fetch_optional(pool)
    .await?;
    maybe.ok_or(BrewAuditError::AlarmNotFound)
}

pub async fn update_status(
    pool: &PgPool,
    alarm_id: Uuid,
    user_id: Uuid,
    next_status: &str,
) -> Result<LongBrewAlarmRow, BrewAuditError> {
    let maybe = sqlx::query_as::<_, LongBrewAlarmRow>(
        r#"
        UPDATE long_brew_scheduled_alarms
        SET alarm_status = $3, updated_at = NOW()
        WHERE id = $1 AND user_id = $2
        RETURNING *
        "#,
    )
    .bind(alarm_id)
    .bind(user_id)
    .bind(next_status)
    .fetch_optional(pool)
    .await?;
    maybe.ok_or(BrewAuditError::AlarmNotFound)
}

pub async fn claim_due_alarms(pool: &PgPool) -> Result<Vec<LongBrewAlarmRow>, BrewAuditError> {
    let rows = sqlx::query_as::<_, LongBrewAlarmRow>(
        r#"
        UPDATE long_brew_scheduled_alarms
        SET
            alarm_status = 'RINGING',
            last_push_sent_at = NOW(),
            updated_at = NOW()
        WHERE alarm_status IN ('ACTIVE', 'SNOOZED')
          AND target_finish_at <= NOW()
        RETURNING *
        "#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
