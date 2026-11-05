use axum::{
    extract::{Path, State},
    Json,
};
use secrecy::SecretString;
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppError, AppResult},
    extractors::{RequireTier, TierBusiness, ValidatedJson},
    security::hash_password,
    AppState,
};
use crate::modules::business_pos::{
    staff_dto::{CreateStaffDto, StaffResponse, UpdateStaffDto},
    staff_repository::BusinessStaffRepository,
    tenant_context::TenantContext,
};

fn parse_user_id(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::Unauthorized("ID Pengguna tidak valid.".to_string()))
}

pub async fn list_staff(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Vec<StaffResponse>>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let tenant = TenantContext::resolve(&state.pool, user_id).await?;

    let staff = BusinessStaffRepository::list_by_store(&state.pool, tenant.store_id).await?;
    let resp: Vec<StaffResponse> = staff.into_iter().map(Into::into).collect();
    Ok(ApiResponse::ok("Daftar staf barista berhasil dimuat.", resp))
}

pub async fn create_staff(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<CreateStaffDto>,
) -> AppResult<Json<ApiResponse<StaffResponse>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let tenant = TenantContext::resolve(&state.pool, user_id).await?;

    let secret = SecretString::from(dto.passcode_pin.clone());
    let pin_hash = hash_password(&secret)?;

    let member = BusinessStaffRepository::create(&state.pool, tenant.store_id, &dto, &pin_hash).await?;
    Ok(ApiResponse::ok("Barista baru berhasil didaftarkan.", member.into()))
}

pub async fn update_staff(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(staff_id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<UpdateStaffDto>,
) -> AppResult<Json<ApiResponse<StaffResponse>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let tenant = TenantContext::resolve(&state.pool, user_id).await?;

    let pin_hash = if let Some(ref pin) = dto.passcode_pin {
        let secret = SecretString::from(pin.clone());
        Some(hash_password(&secret)?)
    } else {
        None
    };

    let updated = BusinessStaffRepository::update(
        &state.pool,
        tenant.store_id,
        staff_id,
        &dto,
        pin_hash.as_deref(),
    )
    .await?;

    Ok(ApiResponse::ok("Data staf barista berhasil diperbarui.", updated.into()))
}

pub async fn delete_staff(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(staff_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<bool>>> {
    let user_id = parse_user_id(&claims.sub)?;
    let tenant = TenantContext::resolve(&state.pool, user_id).await?;

    let success = BusinessStaffRepository::delete(&state.pool, tenant.store_id, staff_id).await?;
    Ok(ApiResponse::ok("Staf barista berhasil dihapus.", success))
}
