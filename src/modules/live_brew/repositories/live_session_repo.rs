use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{
    address_cipher::encode_dynamic_session_code,
    dto::CreateLiveBrewSessionRequest,
    models::LiveBrewSessionRow,
    protocol::LiveBrewStage,
    repositories::brew_table_repo::fetch_user_store_info,
};

const SELECT_COLS: &str = "id, seq_num, dynamic_address_code, store_id, table_id, pos_order_id, barista_user_id, barista_display_name, customer_user_id, customer_display_name, recipe_id, recipe_title, brew_method, bean_name, roastery_name, bean_origin, bean_process, tasting_notes, target_dose_g, target_water_ml, target_temp_c, secondary_temp_c, target_duration_sec, recipe_steps_json, current_stage, current_step_index, elapsed_sec, current_weight_g, current_flow_rate_ml_s, current_temp_c, is_video_stream_active, webrtc_whep_endpoint, recorded_telemetry_curve, final_accuracy_score_pct, cheers_reactions_count, viewer_peak_count, started_at, completed_at, created_at";

pub async fn create_session(
    pool: &PgPool,
    barista_id: Uuid,
    barista_name: &str,
    req: &CreateLiveBrewSessionRequest,
    cipher_secret: &str,
) -> Result<LiveBrewSessionRow, sqlx::Error> {
    let (store_id, _) = fetch_user_store_info(pool, barista_id).await;
    let id = Uuid::now_v7();
    let seed = ((id.as_u128() ^ (barista_id.as_u128() >> 32)) & 0x3FFFFFFFF) as u64 + 1;
    let dynamic_code = encode_dynamic_session_code(seed, cipher_secret);
    let tasting_json = serde_json::to_value(
        req.tasting_notes.as_deref().unwrap_or(&["Sweet", "Floral", "Clean"].map(str::to_string)),
    )
    .unwrap_or_default();
    let steps_json = req.recipe_steps.clone().unwrap_or_else(|| serde_json::json!([]));
    let target_temp = req.target_temp_c.unwrap_or(91.0);

    let sql = format!(
        "INSERT INTO live_brew_sessions (id, dynamic_address_code, store_id, table_id, pos_order_id, barista_user_id, barista_display_name, customer_display_name, recipe_id, recipe_title, brew_method, bean_name, roastery_name, bean_origin, bean_process, tasting_notes, target_dose_g, target_water_ml, target_temp_c, secondary_temp_c, target_duration_sec, recipe_steps_json, current_temp_c, started_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,NOW()) RETURNING {SELECT_COLS}"
    );

    sqlx::query_as::<_, LiveBrewSessionRow>(&sql)
        .bind(id)
        .bind(dynamic_code)
        .bind(store_id)
        .bind(req.table_id)
        .bind(req.pos_order_id)
        .bind(barista_id)
        .bind(barista_name)
        .bind(req.customer_display_name.as_deref().unwrap_or("Pelanggan Spesial"))
        .bind(req.recipe_id)
        .bind(req.recipe_title.trim())
        .bind(req.brew_method.as_deref().unwrap_or("V60"))
        .bind(req.bean_name.as_deref().unwrap_or("Malabar Wine Reserve"))
        .bind(req.roastery_name.as_deref().unwrap_or("Coffee Labs Slow Bar"))
        .bind(req.bean_origin.as_deref().unwrap_or("Jawa Barat, 1500 MDPL"))
        .bind(req.bean_process.as_deref().unwrap_or("Wine Natural"))
        .bind(tasting_json)
        .bind(req.target_dose_g.unwrap_or(15.0))
        .bind(req.target_water_ml.unwrap_or(225.0))
        .bind(target_temp)
        .bind(req.secondary_temp_c)
        .bind(req.target_duration_sec.unwrap_or(145))
        .bind(steps_json)
        .bind(target_temp)
        .fetch_one(pool)
        .await
}

pub async fn fetch_by_id(pool: &PgPool, session_id: Uuid) -> Result<Option<LiveBrewSessionRow>, sqlx::Error> {
    let sql = format!("SELECT {SELECT_COLS} FROM live_brew_sessions WHERE id = $1");
    sqlx::query_as::<_, LiveBrewSessionRow>(&sql).bind(session_id).fetch_optional(pool).await
}

pub async fn fetch_by_dynamic_code(pool: &PgPool, code: &str) -> Result<Option<LiveBrewSessionRow>, sqlx::Error> {
    let sql = format!("SELECT {SELECT_COLS} FROM live_brew_sessions WHERE dynamic_address_code = $1");
    sqlx::query_as::<_, LiveBrewSessionRow>(&sql).bind(code).fetch_optional(pool).await
}

pub async fn update_live_stage_snapshot(
    pool: &PgPool,
    session_id: Uuid,
    stage: LiveBrewStage,
    step_index: i16,
    temp_c: f32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE live_brew_sessions SET current_stage = $2::text::live_brew_stage, current_step_index = $3, current_temp_c = $4, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(stage.as_db_str())
        .bind(step_index)
        .bind(temp_c)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_telemetry_checkpoint(
    pool: &PgPool,
    session_id: Uuid,
    elapsed_sec: i32,
    weight_g: f32,
    flow_ml_s: f32,
    temp_c: f32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE live_brew_sessions SET elapsed_sec = $2, current_weight_g = $3, current_flow_rate_ml_s = $4, current_temp_c = $5, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(elapsed_sec)
        .bind(weight_g)
        .bind(flow_ml_s)
        .bind(temp_c)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_video_stream_flag(pool: &PgPool, session_id: Uuid, active: bool, whep_url: Option<&str>) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE live_brew_sessions SET is_video_stream_active = $2, webrtc_whep_endpoint = $3, updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(active)
        .bind(whep_url)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn increment_cheers(pool: &PgPool, session_id: Uuid) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        "UPDATE live_brew_sessions SET cheers_reactions_count = cheers_reactions_count + 1 WHERE id = $1 RETURNING cheers_reactions_count",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
}

pub async fn archive_completed_session(
    pool: &PgPool,
    session_id: Uuid,
    final_weight_g: f32,
    total_duration_sec: i32,
    accuracy_score_pct: f32,
    curve_json: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE live_brew_sessions SET current_stage = 'READY_TO_SERVE', current_weight_g = $2, elapsed_sec = $3, final_accuracy_score_pct = $4, recorded_telemetry_curve = $5, completed_at = NOW(), updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(final_weight_g)
        .bind(total_duration_sec)
        .bind(accuracy_score_pct)
        .bind(curve_json)
        .execute(pool)
        .await?;
    Ok(())
}
