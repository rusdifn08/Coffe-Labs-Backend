use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{
    address_cipher::encode_dynamic_session_code,
    error::LiveBrewError,
    mirror_dto::{CreateRealMirrorSessionRequest, LiveSessionMirrorSnapshot},
    mirror_models::LiveBrewRealSessionRow,
};

const REAL_SESSION_COLS: &str = r#"
    id, share_code, is_business_session, store_id, seat_id,
    pos_order_id, pos_order_number, pos_order_status,
    brewer_user_id, brewer_display_name, brewer_avatar_url, customer_name,
    phase::text AS phase, state_version,
    selected_method_id, selected_method_name, selected_recipe_id,
    selected_recipe_title, recipe_snapshot_json,
    selected_bean_id, bean_snapshot_json,
    timer_status::text AS timer_status, timer_accumulated_ms,
    timer_anchor_server_ms, current_step_index, current_weight_g,
    current_flow_rate_ml_s, current_temp_c, telemetry_curve_json,
    cheers_count, active_viewers_count
"#;

pub async fn create_real_session(
    pool: &PgPool,
    brewer_user_id: Uuid,
    brewer_display_name: &str,
    req: &CreateRealMirrorSessionRequest,
    secret: &str,
    is_business: bool,
    store_id: Option<Uuid>,
) -> Result<LiveBrewRealSessionRow, LiveBrewError> {
    let id = Uuid::now_v7();
    let seq_seed: u64 = (chrono::Utc::now().timestamp_micros().unsigned_abs() % 90_000_000_000) + 1;
    let share_code = encode_dynamic_session_code(seq_seed, secret);
    let initial_phase = if req.selected_recipe_title.is_some() && req.bean_snapshot_json.is_some() {
        "PRE_BREW_CALIBRATED"
    } else if req.selected_recipe_title.is_some() {
        "BREWER_SELECTING_BEAN"
    } else {
        "BREWER_SELECTING_RECIPE"
    };

    let q = format!(
        r#"INSERT INTO live_brew_real_sessions (
                id, share_code, is_business_session, store_id, seat_id,
                pos_order_id, pos_order_number, pos_order_status,
                brewer_user_id, brewer_display_name, customer_name,
                phase, state_version, selected_method_id, selected_method_name,
                selected_recipe_id, selected_recipe_title, recipe_snapshot_json,
                selected_bean_id, bean_snapshot_json
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,
                $12::live_brew_mirror_phase, 1, $13,$14,$15,$16,$17,$18,$19
            )
            RETURNING {}"#,
        REAL_SESSION_COLS
    );

    let row = sqlx::query_as::<_, LiveBrewRealSessionRow>(&q)
        .bind(id)
        .bind(&share_code)
        .bind(is_business)
        .bind(store_id)
        .bind(req.seat_id)
        .bind(req.pos_order_id)
        .bind(&req.pos_order_number)
        .bind(req.pos_order_number.as_ref().map(|_| "BREWING"))
        .bind(brewer_user_id)
        .bind(brewer_display_name)
        .bind(req.customer_name.as_deref().unwrap_or("Tamu Coffee Labs"))
        .bind(initial_phase)
        .bind(&req.selected_method_id)
        .bind(&req.selected_method_name)
        .bind(req.selected_recipe_id)
        .bind(&req.selected_recipe_title)
        .bind(req.recipe_snapshot_json.clone().unwrap_or_else(|| serde_json::json!({})))
        .bind(req.selected_bean_id)
        .bind(req.bean_snapshot_json.clone().unwrap_or_else(|| serde_json::json!({})))
        .fetch_one(pool)
        .await?;

    if let Some(seat_id) = req.seat_id {
        let _ = sqlx::query(
            r#"UPDATE business_cafe_seats
               SET active_live_session_id = $2,
                   active_pos_order_status = COALESCE($3, active_pos_order_status),
                   updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(seat_id)
        .bind(row.id)
        .bind(req.pos_order_number.as_ref().map(|_| "BREWING"))
        .execute(pool)
        .await;
    }

    Ok(row)
}

pub async fn fetch_by_id(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<LiveBrewRealSessionRow>, LiveBrewError> {
    let q = format!("SELECT {} FROM live_brew_real_sessions WHERE id = $1", REAL_SESSION_COLS);
    Ok(sqlx::query_as::<_, LiveBrewRealSessionRow>(&q).bind(session_id).fetch_optional(pool).await?)
}

pub async fn fetch_by_share_code(
    pool: &PgPool,
    share_code: &str,
) -> Result<Option<LiveBrewRealSessionRow>, LiveBrewError> {
    let q = format!(
        "SELECT {} FROM live_brew_real_sessions WHERE share_code = $1",
        REAL_SESSION_COLS
    );
    Ok(sqlx::query_as::<_, LiveBrewRealSessionRow>(&q)
        .bind(share_code.trim())
        .fetch_optional(pool)
        .await?)
}

pub async fn persist_snapshot_state(
    pool: &PgPool,
    snap: &LiveSessionMirrorSnapshot,
) -> Result<(), LiveBrewError> {
    sqlx::query(
        r#"UPDATE live_brew_real_sessions
           SET phase = $2::live_brew_mirror_phase,
               state_version = $3,
               selected_method_id = $4,
               selected_method_name = $5,
               selected_recipe_id = $6,
               selected_recipe_title = $7,
               recipe_snapshot_json = $8,
               selected_bean_id = $9,
               bean_snapshot_json = $10,
               timer_status = $11::live_timer_status,
               timer_accumulated_ms = $12,
               timer_anchor_server_ms = $13,
               current_step_index = $14,
               current_weight_g = $15,
               current_flow_rate_ml_s = $16,
               current_temp_c = $17,
               pos_order_status = $18,
               completed_at = CASE WHEN $2 = 'SERVING_READY_PASSPORT' THEN NOW() ELSE completed_at END,
               updated_at = NOW()
           WHERE id = $1 AND state_version < $3"#,
    )
    .bind(snap.session_id)
    .bind(snap.phase.as_db_str())
    .bind(snap.state_version)
    .bind(&snap.selected_method_id)
    .bind(&snap.selected_method_name)
    .bind(snap.selected_recipe_id)
    .bind(&snap.selected_recipe_title)
    .bind(&snap.recipe_snapshot_json)
    .bind(snap.selected_bean_id)
    .bind(&snap.bean_snapshot_json)
    .bind(snap.timer_status.as_db_str())
    .bind(snap.timer_accumulated_ms)
    .bind(snap.timer_anchor_server_ms)
    .bind(snap.current_step_index)
    .bind(snap.current_weight_g)
    .bind(snap.current_flow_rate_ml_s)
    .bind(snap.current_temp_c)
    .bind(&snap.pos_order_status)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn increment_cheers(pool: &PgPool, session_id: Uuid) -> Result<i32, LiveBrewError> {
    let total = sqlx::query_scalar::<_, i32>(
        "UPDATE live_brew_real_sessions SET cheers_count = cheers_count + 1, updated_at = NOW() WHERE id = $1 RETURNING cheers_count",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;
    Ok(total)
}

pub use crate::modules::live_brew::repositories::seat_repo::{
    count_pos_queue_ahead, update_seat_pos_binding,
};
