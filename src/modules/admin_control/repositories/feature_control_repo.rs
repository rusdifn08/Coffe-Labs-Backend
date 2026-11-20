use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    core::error::AppResult,
    modules::admin_control::models::{ControlMetaRow, FeatureControlRow},
};

const FEATURE_COLS: &str = r#"
    feature_key, display_name, group_section, route_prefix, api_path_prefixes,
    display_order, is_locked, lock_status::text AS lock_status,
    custom_badge_label, lock_title, lock_message, estimated_unlock_at,
    allow_admin_bypass, updated_at
"#;

pub async fn fetch_control_meta(pool: &PgPool) -> AppResult<ControlMetaRow> {
    let row = sqlx::query_as::<_, ControlMetaRow>(
        "SELECT config_version, global_maintenance_mode, global_maintenance_message, updated_at
         FROM system_control_meta WHERE singleton_id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.unwrap_or(ControlMetaRow {
        config_version: 1,
        global_maintenance_mode: false,
        global_maintenance_message: None,
        updated_at: Utc::now(),
    }))
}

pub async fn list_all_features(pool: &PgPool) -> AppResult<Vec<FeatureControlRow>> {
    let sql = format!(
        "SELECT {FEATURE_COLS} FROM toolkit_feature_controls
         ORDER BY display_order ASC, feature_key ASC"
    );
    Ok(sqlx::query_as::<_, FeatureControlRow>(&sql).fetch_all(pool).await?)
}

pub async fn find_feature_by_key_tx(
    tx: &mut Transaction<'_, Postgres>,
    feature_key: &str,
) -> AppResult<Option<FeatureControlRow>> {
    let sql = format!(
        "SELECT {FEATURE_COLS} FROM toolkit_feature_controls WHERE feature_key = $1 FOR UPDATE"
    );
    Ok(sqlx::query_as::<_, FeatureControlRow>(&sql)
        .bind(feature_key)
        .fetch_optional(&mut **tx)
        .await?)
}

pub struct UpdateFeatureParams<'a> {
    pub feature_key: &'a str,
    pub is_locked: bool,
    pub lock_status: &'a str,
    pub custom_badge_label: Option<&'a str>,
    pub lock_title: &'a str,
    pub lock_message: &'a str,
    pub estimated_unlock_at: Option<DateTime<Utc>>,
    pub allow_admin_bypass: bool,
    pub admin_user_id: Uuid,
}

pub async fn update_feature_lock_tx(
    tx: &mut Transaction<'_, Postgres>,
    p: UpdateFeatureParams<'_>,
) -> AppResult<FeatureControlRow> {
    let sql = format!(
        "UPDATE toolkit_feature_controls
         SET is_locked = $2,
             lock_status = $3::feature_lock_status_kind,
             custom_badge_label = $4,
             lock_title = $5,
             lock_message = $6,
             estimated_unlock_at = $7,
             allow_admin_bypass = $8,
             updated_by_admin_id = $9,
             updated_at = NOW()
         WHERE feature_key = $1
         RETURNING {FEATURE_COLS}"
    );
    Ok(sqlx::query_as::<_, FeatureControlRow>(&sql)
        .bind(p.feature_key)
        .bind(p.is_locked)
        .bind(p.lock_status)
        .bind(p.custom_badge_label)
        .bind(p.lock_title)
        .bind(p.lock_message)
        .bind(p.estimated_unlock_at)
        .bind(p.allow_admin_bypass)
        .bind(p.admin_user_id)
        .fetch_one(&mut **tx)
        .await?)
}

pub struct BulkUpdateFeaturesParams<'a> {
    pub feature_keys: Option<&'a [String]>,
    pub is_locked: bool,
    pub lock_status: &'a str,
    pub custom_badge_label: Option<&'a str>,
    pub lock_title: &'a str,
    pub lock_message: &'a str,
    pub estimated_unlock_at: Option<DateTime<Utc>>,
    pub allow_admin_bypass: bool,
    pub admin_user_id: Uuid,
}

pub async fn bulk_update_features_tx(
    tx: &mut Transaction<'_, Postgres>,
    p: BulkUpdateFeaturesParams<'_>,
) -> AppResult<u64> {
    let res = if let Some(keys) = p.feature_keys {
        sqlx::query(
            "UPDATE toolkit_feature_controls
             SET is_locked = $1,
                 lock_status = $2::feature_lock_status_kind,
                 custom_badge_label = $3,
                 lock_title = $4,
                 lock_message = $5,
                 estimated_unlock_at = $6,
                 allow_admin_bypass = $7,
                 updated_by_admin_id = $8,
                 updated_at = NOW()
             WHERE feature_key = ANY($9)",
        )
        .bind(p.is_locked)
        .bind(p.lock_status)
        .bind(p.custom_badge_label)
        .bind(p.lock_title)
        .bind(p.lock_message)
        .bind(p.estimated_unlock_at)
        .bind(p.allow_admin_bypass)
        .bind(p.admin_user_id)
        .bind(keys)
        .execute(&mut **tx)
        .await?
    } else {
        sqlx::query(
            "UPDATE toolkit_feature_controls
             SET is_locked = $1,
                 lock_status = $2::feature_lock_status_kind,
                 custom_badge_label = $3,
                 lock_title = $4,
                 lock_message = $5,
                 estimated_unlock_at = $6,
                 allow_admin_bypass = $7,
                 updated_by_admin_id = $8,
                 updated_at = NOW()",
        )
        .bind(p.is_locked)
        .bind(p.lock_status)
        .bind(p.custom_badge_label)
        .bind(p.lock_title)
        .bind(p.lock_message)
        .bind(p.estimated_unlock_at)
        .bind(p.allow_admin_bypass)
        .bind(p.admin_user_id)
        .execute(&mut **tx)
        .await?
    };
    Ok(res.rows_affected())
}

pub async fn bump_config_version_tx(
    tx: &mut Transaction<'_, Postgres>,
    admin_user_id: Uuid,
) -> AppResult<i64> {
    let next_ver: i64 = sqlx::query_scalar(
        "INSERT INTO system_control_meta (singleton_id, config_version, updated_by_admin_id, updated_at)
         VALUES (1, 2, $1, NOW())
         ON CONFLICT (singleton_id) DO UPDATE
         SET config_version = system_control_meta.config_version + 1,
             updated_by_admin_id = EXCLUDED.updated_by_admin_id,
             updated_at = NOW()
         RETURNING config_version",
    )
    .bind(admin_user_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(next_ver)
}
