use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        AppState,
    },
    modules::admin_control::{
        dto::{BulkFeatureLockDto, ControlSnapshotDto, UpdateFeatureLockDto},
        middleware::admin_role_guard::RequireAdminUser,
        services::control_mutation_service,
    },
};

pub async fn list_admin_features(
    _admin: RequireAdminUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<ControlSnapshotDto>>> {
    let snap = state.admin_control.reload_from_db(&state.pool).await?;
    Ok(ApiResponse::ok(
        "Daftar kontrol kartu Barista Toolkit diperoleh.",
        snap,
    ))
}

pub async fn update_feature_lock(
    admin: RequireAdminUser,
    State(state): State<AppState>,
    Path(feature_key): Path<String>,
    Json(req): Json<UpdateFeatureLockDto>,
) -> AppResult<Json<ApiResponse<ControlSnapshotDto>>> {
    let snap = control_mutation_service::update_single_feature_lock(
        &state,
        admin.user_id,
        &admin.admin_label,
        &feature_key,
        req,
    )
    .await?;
    Ok(ApiResponse::ok(
        format!("Status kartu {feature_key} berhasil disinkronkan real-time ke semua akun."),
        snap,
    ))
}

pub async fn bulk_lock_features(
    admin: RequireAdminUser,
    State(state): State<AppState>,
    Json(req): Json<BulkFeatureLockDto>,
) -> AppResult<Json<ApiResponse<Value>>> {
    let (affected, snap) = control_mutation_service::bulk_update_feature_locks(
        &state,
        admin.user_id,
        &admin.admin_label,
        req,
    )
    .await?;
    Ok(ApiResponse::ok(
        format!("{affected} kartu Barista Toolkit diperbarui dan disiarkan real-time."),
        json!({
            "affected_rows": affected,
            "snapshot": snap
        }),
    ))
}
