use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};

pub use crate::modules::monitoring::repositories::dossier_rollup_repo::PendingRawEvent;
use crate::modules::monitoring::repositories::dossier_rollup_repo::{
    upsert_daily_rollups_and_extract_brews, upsert_dossier_deltas, PendingBrewExp,
};

/// Mem-flush seluruh event batch dalam satu transaksi PostgreSQL berbasis UNNEST (ramah NeonDB & 512MB RAM).
pub async fn flush_telemetry_batch(
    pool: &PgPool,
    events: &[PendingRawEvent],
) -> Result<(), sqlx::Error> {
    if events.is_empty() {
        return Ok(());
    }

    let today = Utc::now().date_naive();
    let mut tx = pool.begin().await?;

    insert_raw_events_unnest(&mut tx, events).await?;
    let brew_exps = upsert_daily_rollups_and_extract_brews(&mut tx, events).await?;
    insert_brew_experiments(&mut tx, &brew_exps).await?;
    upsert_dossier_deltas(&mut tx, events, today).await?;

    tx.commit().await
}

async fn insert_raw_events_unnest(
    tx: &mut Transaction<'_, Postgres>,
    events: &[PendingRawEvent],
) -> Result<(), sqlx::Error> {
    let mut event_ids = Vec::with_capacity(events.len());
    let mut user_ids = Vec::with_capacity(events.len());
    let mut session_ids = Vec::with_capacity(events.len());
    let mut types = Vec::with_capacity(events.len());
    let mut buckets = Vec::with_capacity(events.len());
    let mut routes = Vec::with_capacity(events.len());
    let mut durations = Vec::with_capacity(events.len());
    let mut metas = Vec::with_capacity(events.len());
    let mut recorded_ats = Vec::with_capacity(events.len());

    for ev in events {
        event_ids.push(ev.event_id);
        user_ids.push(ev.user_id);
        session_ids.push(ev.session_id);
        types.push(ev.event_type.as_str());
        buckets.push(ev.feature_bucket.as_str());
        routes.push(ev.route_name.as_str());
        durations.push(ev.active_screen_sec + ev.aod_kiosk_sec);
        metas.push(ev.metadata_json.clone());
        recorded_ats.push(ev.recorded_at);
    }

    sqlx::query(
        r#"INSERT INTO telemetry_raw_events (
               event_id, user_id, session_id, event_type, feature_bucket,
               route_name, duration_sec, metadata_json, recorded_at
           )
           SELECT * FROM UNNEST(
               $1::uuid[], $2::uuid[], $3::uuid[], $4::varchar[],
               $5::varchar[], $6::varchar[], $7::int[], $8::jsonb[], $9::timestamptz[]
           )
           ON CONFLICT (event_id, recorded_at) DO NOTHING"#,
    )
    .bind(&event_ids)
    .bind(&user_ids)
    .bind(&session_ids)
    .bind(&types)
    .bind(&buckets)
    .bind(&routes)
    .bind(&durations)
    .bind(&metas)
    .bind(&recorded_ats)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_brew_experiments(
    tx: &mut Transaction<'_, Postgres>,
    brews: &[PendingBrewExp],
) -> Result<(), sqlx::Error> {
    for b in brews {
        sqlx::query(
            r#"INSERT INTO account_brew_experiment_logs (
                   user_id, session_id, recipe_id, recipe_title, brew_method,
                   bean_origin, brew_duration_sec, completed, steps_completed, rating, created_at
               ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
        )
        .bind(b.user_id)
        .bind(b.session_id)
        .bind(&b.recipe_id)
        .bind(&b.recipe_title)
        .bind(&b.brew_method)
        .bind(&b.bean_origin)
        .bind(b.duration_sec)
        .bind(b.completed)
        .bind(b.steps_completed)
        .bind(b.rating)
        .bind(b.created_at)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
