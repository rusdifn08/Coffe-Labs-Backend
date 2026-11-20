use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{error::AppResult, AppState},
    modules::admin_control::{
        dto::{ControlSnapshotDto, UpdatePlanPriceDto},
        error::AdminControlError,
        repositories::{
            feature_control_repo,
            pricing_control_repo::{self, NewAuditLogParams, UpdatePlanParams},
        },
    },
};

pub async fn update_plan_pricing(
    state: &AppState,
    admin_user_id: Uuid,
    admin_fallback_label: &str,
    plan_code_raw: &str,
    req: UpdatePlanPriceDto,
) -> AppResult<ControlSnapshotDto> {
    if req.base_price_idr < 1_000 || req.base_price_idr > 500_000_000 {
        return Err(AdminControlError::InvalidPriceConfig(
            "Harga jual wajib antara Rp 1.000 hingga Rp 500.000.000.".into(),
        )
        .into());
    }
    if req.duration_days < 1 || req.duration_days > 365_000 {
        return Err(AdminControlError::InvalidPriceConfig(
            "Durasi paket wajib antara 1 hingga 365.000 hari.".into(),
        )
        .into());
    }
    if let Some(orig) = req.original_price_idr {
        if orig > 0 && orig < req.base_price_idr {
            return Err(AdminControlError::InvalidPriceConfig(
                "Harga coret promo tidak boleh lebih kecil dari harga jual aktif.".into(),
            )
            .into());
        }
    }

    let plan_code = plan_code_raw.trim().to_ascii_uppercase();
    let admin_email =
        pricing_control_repo::resolve_admin_email(&state.pool, admin_user_id, admin_fallback_label)
            .await;

    let mut tx = state.pool.begin().await?;
    let existing = pricing_control_repo::find_plan_by_code_tx(&mut tx, &plan_code)
        .await?
        .ok_or_else(|| AdminControlError::PlanNotFound(plan_code.clone()))?;

    let prev_json = json!({
        "display_name": existing.display_name,
        "base_price_idr": existing.base_price_idr,
        "original_price_idr": existing.original_price_idr,
        "promo_badge_text": existing.promo_badge_text,
        "duration_days": existing.duration_days,
        "is_active": existing.is_active
    });

    let display_name = req
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&existing.display_name);
    let promo_badge = req
        .promo_badge_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let original_price = req.original_price_idr.filter(|v| *v > 0);

    let updated = pricing_control_repo::update_plan_pricing_tx(
        &mut tx,
        UpdatePlanParams {
            plan_code: &plan_code,
            display_name,
            base_price_idr: req.base_price_idr,
            original_price_idr: original_price,
            promo_badge_text: promo_badge,
            duration_days: req.duration_days,
            is_active: req.is_active.unwrap_or(existing.is_active),
            is_highlighted: req.is_highlighted.unwrap_or(existing.is_highlighted),
            admin_user_id,
        },
    )
    .await?;

    let next_version = feature_control_repo::bump_config_version_tx(&mut tx, admin_user_id).await?;
    let new_json = json!({
        "display_name": updated.display_name,
        "base_price_idr": updated.base_price_idr,
        "original_price_idr": updated.original_price_idr,
        "promo_badge_text": updated.promo_badge_text,
        "duration_days": updated.duration_days,
        "is_active": updated.is_active
    });

    pricing_control_repo::insert_audit_log_tx(
        &mut tx,
        NewAuditLogParams {
            admin_user_id,
            admin_email: &admin_email,
            action_category: "PLAN_PRICE_UPDATE",
            target_key: &plan_code,
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
