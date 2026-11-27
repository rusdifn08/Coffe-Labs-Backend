use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::live_brew::{
    error::LiveBrewError,
    mirror_models::CafeSeatRow,
    mirror_protocol::CafeSeatKind,
};

const SEAT_COLS: &str = r#"
    id, store_id, seat_kind::text AS seat_kind, seat_label, seat_slug,
    zone_area, capacity_persons, numeric_fallback_code,
    active_live_session_id, active_pos_order_id, active_pos_order_number,
    active_pos_order_status, active_pos_items_summary, active_pos_customer_name,
    qr_scan_count, is_active, updated_at
"#;

pub async fn create_or_upsert_seat(
    pool: &PgPool,
    store_id: Uuid,
    seat_kind: CafeSeatKind,
    seat_label: &str,
    seat_slug: &str,
    zone_area: &str,
    capacity: i16,
    fallback_code: &str,
) -> Result<CafeSeatRow, LiveBrewError> {
    let q = format!(
        r#"INSERT INTO business_cafe_seats (
                id, store_id, seat_kind, seat_label, seat_slug,
                zone_area, capacity_persons, numeric_fallback_code
            ) VALUES ($1, $2, $3::cafe_seat_kind, $4, $5, $6, $7, $8)
            ON CONFLICT (store_id, seat_slug) DO UPDATE SET
                seat_label = EXCLUDED.seat_label,
                seat_kind = EXCLUDED.seat_kind,
                zone_area = EXCLUDED.zone_area,
                capacity_persons = EXCLUDED.capacity_persons,
                is_active = TRUE,
                updated_at = NOW()
            RETURNING {}"#,
        SEAT_COLS
    );
    let row = sqlx::query_as::<_, CafeSeatRow>(&q)
        .bind(Uuid::now_v7())
        .bind(store_id)
        .bind(seat_kind.as_db_str())
        .bind(seat_label.trim())
        .bind(seat_slug)
        .bind(zone_area.trim())
        .bind(capacity.clamp(1, 30))
        .bind(fallback_code)
        .fetch_one(pool)
        .await?;
    Ok(row)
}

pub async fn list_seats_for_store(
    pool: &PgPool,
    store_id: Uuid,
) -> Result<Vec<CafeSeatRow>, LiveBrewError> {
    let q = format!(
        "SELECT {} FROM business_cafe_seats WHERE store_id = $1 AND is_active = TRUE ORDER BY zone_area ASC, seat_label ASC",
        SEAT_COLS
    );
    Ok(sqlx::query_as::<_, CafeSeatRow>(&q).bind(store_id).fetch_all(pool).await?)
}

pub async fn fetch_seat_by_id(
    pool: &PgPool,
    seat_id: Uuid,
) -> Result<Option<CafeSeatRow>, LiveBrewError> {
    let q = format!("SELECT {} FROM business_cafe_seats WHERE id = $1 AND is_active = TRUE", SEAT_COLS);
    Ok(sqlx::query_as::<_, CafeSeatRow>(&q).bind(seat_id).fetch_optional(pool).await?)
}

pub async fn fetch_seat_by_store_and_slug(
    pool: &PgPool,
    store_id: Uuid,
    seat_slug: &str,
) -> Result<Option<CafeSeatRow>, LiveBrewError> {
    let q = format!(
        "SELECT {} FROM business_cafe_seats WHERE store_id = $1 AND seat_slug = $2 AND is_active = TRUE",
        SEAT_COLS
    );
    Ok(sqlx::query_as::<_, CafeSeatRow>(&q)
        .bind(store_id)
        .bind(seat_slug.trim().to_ascii_lowercase())
        .fetch_optional(pool)
        .await?)
}

pub async fn fetch_seat_by_fallback_code(
    pool: &PgPool,
    code: &str,
) -> Result<Option<CafeSeatRow>, LiveBrewError> {
    let q = format!(
        "SELECT {} FROM business_cafe_seats WHERE numeric_fallback_code = $1 AND is_active = TRUE",
        SEAT_COLS
    );
    Ok(sqlx::query_as::<_, CafeSeatRow>(&q).bind(code.trim()).fetch_optional(pool).await?)
}

pub async fn update_seat_pos_binding(
    pool: &PgPool,
    seat_id: Uuid,
    pos_order_id: Option<Uuid>,
    pos_order_number: &str,
    pos_order_status: &str,
    customer_name: &str,
    items_summary: &str,
) -> Result<(), LiveBrewError> {
    sqlx::query(
        r#"UPDATE business_cafe_seats
           SET active_pos_order_id = $2,
               active_pos_order_number = $3,
               active_pos_order_status = $4,
               active_pos_customer_name = $5,
               active_pos_items_summary = $6,
               updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(seat_id)
    .bind(pos_order_id)
    .bind(pos_order_number)
    .bind(pos_order_status)
    .bind(customer_name)
    .bind(items_summary)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count_pos_queue_ahead(
    pool: &PgPool,
    store_id: Uuid,
    seat_id: Uuid,
) -> Result<i64, LiveBrewError> {
    let cnt = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM business_cafe_seats
           WHERE store_id = $1
             AND id <> $2
             AND active_pos_order_status IN ('QUEUED', 'PREPARING', 'BREWING')"#,
    )
    .bind(store_id)
    .bind(seat_id)
    .fetch_one(pool)
    .await?;
    Ok(cnt)
}

pub async fn deactivate_seat(
    pool: &PgPool,
    store_id: Uuid,
    seat_id: Uuid,
) -> Result<bool, LiveBrewError> {
    let res = sqlx::query(
        r#"UPDATE business_cafe_seats
           SET is_active = FALSE,
               active_live_session_id = NULL,
               updated_at = NOW()
           WHERE id = $1 AND store_id = $2 AND is_active = TRUE"#,
    )
    .bind(seat_id)
    .bind(store_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected() > 0)
}
