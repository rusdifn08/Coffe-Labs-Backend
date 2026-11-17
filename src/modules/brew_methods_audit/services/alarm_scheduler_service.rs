use chrono::{Duration, Utc};
use fred::interfaces::SortedSetsInterface;
use fred::prelude::RedisPool;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::brew_methods_audit::{
    coldbrew_math::compute_cold_brew_plan,
    dto::{LongAlarmDto, ScheduleLongAlarmRequest},
    error::BrewAuditError,
    models::LongBrewAlarmRow,
    repositories::long_alarm_repo,
};

pub const BREW_ALARM_ZSET_KEY: &str = "brew:alarms:due_zset";

pub fn alarm_row_to_dto(row: &LongBrewAlarmRow) -> LongAlarmDto {
    let remaining = (row.target_finish_at - Utc::now()).num_seconds().max(0);
    LongAlarmDto {
        id: row.id,
        recipe_id: row.recipe_id,
        recipe_title: row.recipe_title.clone(),
        method_slug: row.method_slug.clone(),
        brew_mode: row.brew_mode.clone(),
        coffee_dose_g: row.coffee_dose_g,
        water_ml: row.water_ml,
        ambient_temp_c: row.ambient_temp_c,
        drip_rate_bpm: row.drip_rate_bpm,
        started_at: row.started_at,
        target_finish_at: row.target_finish_at,
        total_duration_sec: row.total_duration_sec,
        remaining_seconds: remaining,
        checkpoints: row.checkpoints_json.clone(),
        alarm_status: row.alarm_status.clone(),
        snooze_count: row.snooze_count,
    }
}

pub async fn schedule_alarm(
    pool: &PgPool,
    redis_pool: Option<&RedisPool>,
    user_id: Uuid,
    req: &ScheduleLongAlarmRequest,
) -> Result<LongAlarmDto, BrewAuditError> {
    if req.recipe_title.trim().is_empty() {
        return Err(BrewAuditError::Validation(
            "Judul resep tidak boleh kosong.".to_string(),
        ));
    }
    if req.duration_seconds < 60 || req.duration_seconds > 172_800 {
        return Err(BrewAuditError::Validation(
            "Durasi alarm Cold Brew harus di antara 60 detik hingga 48 jam.".to_string(),
        ));
    }
    if req.coffee_dose_g <= 0.0 || req.water_ml <= 0.0 {
        return Err(BrewAuditError::Validation(
            "Dosis kopi dan volume air harus bernilai positif.".to_string(),
        ));
    }

    let plan = compute_cold_brew_plan(
        &req.brew_mode,
        req.coffee_dose_g,
        req.water_ml,
        req.ambient_temp_c.unwrap_or(4.0),
        req.drip_rate_bpm,
    );
    let checkpoints_json = serde_json::to_value(&plan.checkpoints).unwrap_or_default();
    let target_finish = Utc::now() + Duration::seconds(i64::from(req.duration_seconds));

    let row = long_alarm_repo::insert_alarm(pool, user_id, req, checkpoints_json, target_finish)
        .await?;
    sync_alarm_zset(redis_pool, row.id, row.target_finish_at.timestamp_millis() as f64).await;
    Ok(alarm_row_to_dto(&row))
}

pub async fn snooze_alarm(
    pool: &PgPool,
    redis_pool: Option<&RedisPool>,
    user_id: Uuid,
    alarm_id: Uuid,
    minutes: i32,
) -> Result<LongAlarmDto, BrewAuditError> {
    let clamped_min = minutes.clamp(5, 180);
    let next_finish = Utc::now() + Duration::minutes(i64::from(clamped_min));
    let row = long_alarm_repo::snooze_alarm(pool, alarm_id, user_id, next_finish).await?;
    sync_alarm_zset(redis_pool, row.id, row.target_finish_at.timestamp_millis() as f64).await;
    Ok(alarm_row_to_dto(&row))
}

pub async fn finish_or_cancel_alarm(
    pool: &PgPool,
    redis_pool: Option<&RedisPool>,
    user_id: Uuid,
    alarm_id: Uuid,
    status: &str,
) -> Result<LongAlarmDto, BrewAuditError> {
    let row = long_alarm_repo::update_status(pool, alarm_id, user_id, status).await?;
    if let Some(pool) = redis_pool {
        let _ = pool
            .zrem::<i64, _, _>(BREW_ALARM_ZSET_KEY, row.id.to_string())
            .await;
    }
    Ok(alarm_row_to_dto(&row))
}

async fn sync_alarm_zset(redis_pool: Option<&RedisPool>, alarm_id: Uuid, fire_ms: f64) {
    if let Some(pool) = redis_pool {
        let _ = pool
            .zadd::<i64, _, _>(
                BREW_ALARM_ZSET_KEY,
                None,
                None,
                false,
                false,
                (fire_ms, alarm_id.to_string()),
            )
            .await;
    }
}
