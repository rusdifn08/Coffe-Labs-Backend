use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::admin_control::{
        dto::{
            normalize_lock_status, BulkFeatureLockDto, ControlSnapshotDto, UpdateFeatureLockDto,
        },
        error::AdminControlError,
        repositories::{
            feature_control_repo::{self, BulkUpdateFeaturesParams, UpdateFeatureParams},
            pricing_control_repo::{self, NewAuditLogParams},
        },
    },
};

pub async fn update_single_feature_lock(
    state: &AppState,
    admin_user_id: Uuid,
    admin_fallback_label: &str,
    feature_key_raw: &str,
    req: UpdateFeatureLockDto,
) -> AppResult<ControlSnapshotDto> {
    let feature_key = feature_key_raw.trim().to_ascii_uppercase();
    let lock_status = normalize_lock_status(&req.lock_status, req.is_locked)
        .ok_or_else(|| AdminControlError::InvalidLockStatus(req.lock_status.clone()))?;

    let admin_email =
        pricing_control_repo::resolve_admin_email(&state.pool, admin_user_id, admin_fallback_label)
            .await;

    let mut tx = state.pool.begin().await?;
    let existing = feature_control_repo::find_feature_by_key_tx(&mut tx, &feature_key)
        .await?
        .ok_or_else(|| AdminControlError::FeatureNotFound(feature_key.clone()))?;

    let prev_json = json!({
        "is_locked": existing.is_locked,
        "lock_status": existing.lock_status,
        "lock_title": existing.lock_title,
        "lock_message": existing.lock_message,
        "allow_admin_bypass": existing.allow_admin_bypass
    });

    let clean_badge = req
        .custom_badge_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let bypass = req.allow_admin_bypass.unwrap_or(existing.allow_admin_bypass);

    let updated = feature_control_repo::update_feature_lock_tx(
        &mut tx,
        UpdateFeatureParams {
            feature_key: &feature_key,
            is_locked: req.is_locked,
            lock_status: &lock_status,
            custom_badge_label: clean_badge,
            lock_title: req.lock_title.trim(),
            lock_message: req.lock_message.trim(),
            estimated_unlock_at: req.estimated_unlock_at,
            allow_admin_bypass: bypass,
            admin_user_id,
        },
    )
    .await?;

    let next_version = feature_control_repo::bump_config_version_tx(&mut tx, admin_user_id).await?;
    let new_json = json!({
        "is_locked": updated.is_locked,
        "lock_status": updated.lock_status,
        "lock_title": updated.lock_title,
        "lock_message": updated.lock_message,
        "allow_admin_bypass": updated.allow_admin_bypass
    });

    pricing_control_repo::insert_audit_log_tx(
        &mut tx,
        NewAuditLogParams {
            admin_user_id,
            admin_email: &admin_email,
            action_category: "FEATURE_LOCK_UPDATE",
            target_key: &feature_key,
            previous_state: prev_json,
            new_state: new_json,
            reason_note: req.reason_note.as_deref(),
            config_version_after: next_version,
        },
    )
    .await?;

    tx.commit().await?;
    let snapshot = state.admin_control.reload_from_db(&state.pool).await?;
    state.admin_control.publish_snapshot(snapshot.clone()).await;
    Ok(snapshot)
}

pub async fn bulk_update_feature_locks(
    state: &AppState,
    admin_user_id: Uuid,
    admin_fallback_label: &str,
    req: BulkFeatureLockDto,
) -> AppResult<(u64, ControlSnapshotDto)> {
    let lock_status = normalize_lock_status(&req.lock_status, req.is_locked)
        .ok_or_else(|| AdminControlError::InvalidLockStatus(req.lock_status.clone()))?;
    let admin_email =
        pricing_control_repo::resolve_admin_email(&state.pool, admin_user_id, admin_fallback_label)
            .await;

    let normalized_keys: Option<Vec<String>> = req.feature_keys.as_ref().map(|keys| {
        keys.iter()
            .map(|k| k.trim().to_ascii_uppercase())
            .filter(|k| !k.is_empty())
            .collect()
    });
    let clean_badge = req
        .custom_badge_label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let mut tx = state.pool.begin().await?;
    let affected = feature_control_repo::bulk_update_features_tx(
        &mut tx,
        BulkUpdateFeaturesParams {
            feature_keys: normalized_keys.as_deref(),
            is_locked: req.is_locked,
            lock_status: &lock_status,
            custom_badge_label: clean_badge,
            lock_title: req.lock_title.trim(),
            lock_message: req.lock_message.trim(),
            estimated_unlock_at: req.estimated_unlock_at,
            allow_admin_bypass: req.allow_admin_bypass.unwrap_or(true),
            admin_user_id,
        },
    )
    .await?;

    let next_version = feature_control_repo::bump_config_version_tx(&mut tx, admin_user_id).await?;
    let target_label = match &normalized_keys {
        Some(keys) => format!("BULK({})", keys.len()),
        None => "ALL_TOOLKIT_CARDS".to_string(),
    };
    pricing_control_repo::insert_audit_log_tx(
        &mut tx,
        NewAuditLogParams {
            admin_user_id,
            admin_email: &admin_email,
            action_category: "BULK_MAINTENANCE_TOGGLE",
            target_key: &target_label,
            previous_state: json!({ "scope": target_label }),
            new_state: json!({
                "is_locked": req.is_locked,
                "lock_status": lock_status,
                "affected_rows": affected
            }),
            reason_note: req.reason_note.as_deref(),
            config_version_after: next_version,
        },
    )
    .await?;

    tx.commit().await?;
    let snapshot = state.admin_control.reload_from_db(&state.pool).await?;
    state.admin_control.publish_snapshot(snapshot.clone()).await;
    Ok((affected, snapshot))
}
