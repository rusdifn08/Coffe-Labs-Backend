use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Default)]
struct DailyUserAgg {
    active_sec: i32,
    aod_sec: i32,
    session_count: i32,
    events_count: i32,
    feature_secs: HashMap<String, i32>,
}

#[derive(Default)]
struct UserDossierDelta {
    today_active_sec: i32,
    today_aod_sec: i32,
    total_active_sec: i32,
    brews_inc: i32,
    scans_inc: i32,
    sessions_inc: i32,
    feature_secs: HashMap<String, i32>,
    last_route: Option<String>,
    last_feature: Option<String>,
    last_seen: Option<DateTime<Utc>>,
    platform: String,
    app_version: Option<String>,
    device_model: Option<String>,
    last_brew_title: Option<String>,
    last_brew_method: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingRawEvent {
    pub event_id: Uuid,
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub event_type: String,
    pub feature_bucket: String,
    pub route_name: String,
    pub active_screen_sec: i32,
    pub aod_kiosk_sec: i32,
    pub metadata_json: serde_json::Value,
    pub recorded_at: DateTime<Utc>,
    pub platform: String,
    pub app_version: Option<String>,
    pub device_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PendingBrewExp {
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub recipe_id: Option<String>,
    pub recipe_title: String,
    pub brew_method: String,
    pub bean_origin: Option<String>,
    pub duration_sec: i32,
    pub completed: bool,
    pub steps_completed: i32,
    pub rating: Option<i16>,
    pub created_at: DateTime<Utc>,
}

pub async fn upsert_daily_rollups_and_extract_brews(
    tx: &mut Transaction<'_, Postgres>,
    events: &[PendingRawEvent],
) -> Result<Vec<PendingBrewExp>, sqlx::Error> {
    let mut agg: HashMap<(Uuid, NaiveDate), DailyUserAgg> = HashMap::new();
    let mut brew_exps = Vec::new();
    for ev in events {
        let e = agg.entry((ev.user_id, ev.recorded_at.date_naive())).or_default();
        e.active_sec += ev.active_screen_sec;
        e.aod_sec += ev.aod_kiosk_sec;
        e.events_count += 1;
        if ev.event_type.eq_ignore_ascii_case("SESSION_START") { e.session_count += 1; }
        let dur = ev.active_screen_sec + ev.aod_kiosk_sec;
        if dur > 0 { *e.feature_secs.entry(ev.feature_bucket.clone()).or_insert(0) += dur; }
        if ev.event_type.eq_ignore_ascii_case("BREW_COMPLETE") {
            let m = &ev.metadata_json;
            brew_exps.push(PendingBrewExp {
                user_id: ev.user_id,
                session_id: ev.session_id,
                recipe_id: m.get("recipe_id").and_then(|v| v.as_str()).map(str::to_string),
                recipe_title: m.get("recipe_title").and_then(|v| v.as_str()).unwrap_or("V60 Manual Brew").chars().take(150).collect(),
                brew_method: m.get("brew_method").and_then(|v| v.as_str()).unwrap_or("V60").chars().take(50).collect(),
                bean_origin: m.get("bean_origin").and_then(|v| v.as_str()).map(|s| s.chars().take(120).collect()),
                duration_sec: ev.active_screen_sec.max(ev.aod_kiosk_sec),
                completed: m.get("completed").and_then(|v| v.as_bool()).unwrap_or(true),
                steps_completed: m.get("steps_completed").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                rating: m.get("rating").and_then(|v| v.as_i64()).map(|r| r as i16),
                created_at: ev.recorded_at,
            });
        }
    }
    for ((uid, date), val) in agg {
        let feat_json = serde_json::to_value(&val.feature_secs).unwrap_or_default();
        sqlx::query(
            r#"INSERT INTO account_daily_telemetry_rollups (user_id, metric_date, active_screen_time_sec, aod_kiosk_time_sec, session_count, feature_seconds_json, events_count)
               VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (user_id, metric_date) DO UPDATE SET
                   active_screen_time_sec = account_daily_telemetry_rollups.active_screen_time_sec + EXCLUDED.active_screen_time_sec,
                   aod_kiosk_time_sec = account_daily_telemetry_rollups.aod_kiosk_time_sec + EXCLUDED.aod_kiosk_time_sec,
                   session_count = account_daily_telemetry_rollups.session_count + EXCLUDED.session_count,
                   feature_seconds_json = (SELECT COALESCE(jsonb_object_agg(k, v), '{}'::jsonb) FROM (SELECT k, SUM(x::bigint)::bigint AS v FROM (SELECT key k, value x FROM jsonb_each_text(COALESCE(account_daily_telemetry_rollups.feature_seconds_json, '{}'::jsonb)) UNION ALL SELECT key k, value x FROM jsonb_each_text(EXCLUDED.feature_seconds_json)) c GROUP BY k) s),
                   events_count = account_daily_telemetry_rollups.events_count + EXCLUDED.events_count"#,
        ).bind(uid).bind(date).bind(val.active_sec).bind(val.aod_sec).bind(val.session_count).bind(feat_json).bind(val.events_count).execute(&mut **tx).await?;
    }
    Ok(brew_exps)
}

pub async fn upsert_dossier_deltas(
    tx: &mut Transaction<'_, Postgres>,
    events: &[PendingRawEvent],
    today: NaiveDate,
) -> Result<(), sqlx::Error> {
    let mut per_user: HashMap<Uuid, UserDossierDelta> = HashMap::new();
    for ev in events {
        let d = per_user.entry(ev.user_id).or_default();
        if ev.recorded_at.date_naive() == today { d.today_active_sec += ev.active_screen_sec; d.today_aod_sec += ev.aod_kiosk_sec; }
        d.total_active_sec += ev.active_screen_sec;
        if ev.event_type.eq_ignore_ascii_case("BREW_COMPLETE") {
            d.brews_inc += 1;
            d.last_brew_title = ev.metadata_json.get("recipe_title").and_then(|v| v.as_str()).map(|s| s.chars().take(150).collect());
            d.last_brew_method = ev.metadata_json.get("brew_method").and_then(|v| v.as_str()).map(|s| s.chars().take(50).collect());
        }
        if ev.event_type.eq_ignore_ascii_case("AI_SCAN") || (ev.feature_bucket == "ai_scanner" && ev.event_type == "FEATURE_ACTION") { d.scans_inc += 1; }
        if ev.event_type.eq_ignore_ascii_case("SESSION_START") { d.sessions_inc += 1; }
        let sec = ev.active_screen_sec + ev.aod_kiosk_sec;
        if sec > 0 { *d.feature_secs.entry(ev.feature_bucket.clone()).or_insert(0) += sec; }
        d.last_route = Some(ev.route_name.clone()); d.last_feature = Some(ev.feature_bucket.clone());
        d.last_seen = Some(ev.recorded_at); d.platform = ev.platform.clone();
        if ev.app_version.is_some() { d.app_version = ev.app_version.clone(); }
        if ev.device_model.is_some() { d.device_model = ev.device_model.clone(); }
    }
    let missing_ids: Vec<Uuid> = per_user.keys().copied().collect();
    sqlx::query(
        r#"INSERT INTO account_monitoring_dossiers (user_id, bitmap_seq_id, username, full_name, role, account_tier, avatar_url)
           SELECT u.id, COALESCE(u.bitmap_seq_id, nextval('user_bitmap_seq')), u.username, u.full_name, u.role, INITCAP(COALESCE(NULLIF(u.tier, ''), 'trial')), u.avatar_url
           FROM users u WHERE u.id = ANY($1) ON CONFLICT (user_id) DO NOTHING"#,
    ).bind(&missing_ids).execute(&mut **tx).await?;
    for (uid, d) in per_user {
        let feat_delta = serde_json::to_value(&d.feature_secs).unwrap_or_default();
        sqlx::query(
            r#"UPDATE account_monitoring_dossiers SET
                   is_online = TRUE, last_seen_at = GREATEST(COALESCE(last_seen_at, $2), $2),
                   current_active_route = COALESCE($3, current_active_route), current_feature_bucket = COALESCE($4, current_feature_bucket),
                   platform = CASE WHEN $5 <> '' THEN $5 ELSE platform END, app_version = COALESCE($6, app_version), device_model = COALESCE($7, device_model),
                   today_screen_time_sec = CASE WHEN updated_at::date = CURRENT_DATE THEN today_screen_time_sec + $8 ELSE $8 END,
                   today_aod_time_sec = CASE WHEN updated_at::date = CURRENT_DATE THEN today_aod_time_sec + $9 ELSE $9 END,
                   total_7d_screen_time_sec = total_7d_screen_time_sec + $10, total_30d_screen_time_sec = total_30d_screen_time_sec + $10,
                   total_lifetime_screen_time_sec = total_lifetime_screen_time_sec + $10, total_sessions_30d = total_sessions_30d + $11,
                   total_brews_executed = total_brews_executed + $12, total_ai_scans = total_ai_scans + $13,
                   last_brewed_recipe_title = COALESCE($14, last_brewed_recipe_title), last_brewed_method = COALESCE($15, last_brewed_method),
                   feature_time_breakdown_json = (SELECT COALESCE(jsonb_object_agg(k, v), '{}'::jsonb) FROM (SELECT k, SUM(x::bigint)::bigint AS v FROM (SELECT key k, value x FROM jsonb_each_text(COALESCE(account_monitoring_dossiers.feature_time_breakdown_json, '{}'::jsonb)) UNION ALL SELECT key k, value x FROM jsonb_each_text($16::jsonb)) c GROUP BY k) s),
                   health_score = LEAST(100, GREATEST(health_score, 72))::smallint, churn_risk_level = 'LOW', updated_at = NOW()
               WHERE user_id = $1"#,
        )
        .bind(uid).bind(d.last_seen.unwrap_or_else(Utc::now)).bind(d.last_route).bind(d.last_feature)
        .bind(d.platform).bind(d.app_version).bind(d.device_model).bind(d.today_active_sec).bind(d.today_aod_sec)
        .bind(d.total_active_sec).bind(d.sessions_inc).bind(d.brews_inc).bind(d.scans_inc)
        .bind(d.last_brew_title).bind(d.last_brew_method).bind(feat_delta)
        .execute(&mut **tx).await?;
    }
    Ok(())
}
