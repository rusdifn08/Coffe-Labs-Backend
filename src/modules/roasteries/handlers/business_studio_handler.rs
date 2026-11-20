use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppResult},
    extractors::{RequireTier, TierBusiness, ValidatedJson},
    storage::CloudinaryUploadSignature,
    AppState,
};
use crate::modules::roasteries::{
    dto::{CreateRoasteryDto, RoasteryUploadSigDto, UpdateRoasteryDto, UpsertLineupDto},
    models::{DeleteLineupOutcome, RoasteryLineupRow, RoasteryWithLineupsResponse},
    services::{cloudinary_asset_service, roastery_studio_service},
};

pub async fn get_my_roastery(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Option<RoasteryWithLineupsResponse>>>> {
    let mine = roastery_studio_service::get_my_roastery(&state, claims.sub).await?;
    Ok(ApiResponse::ok("Status Roastery Business berhasil diambil.", mine))
}

pub async fn create_my_roastery(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<CreateRoasteryDto>,
) -> AppResult<(StatusCode, Json<ApiResponse<RoasteryWithLineupsResponse>>)> {
    let created = roastery_studio_service::create_my_roastery(&state, claims.sub, &dto).await?;
    Ok(ApiResponse::created("Roastery Anda resmi terdaftar & dipublikasikan.", created))
}

pub async fn update_my_roastery(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<UpdateRoasteryDto>,
) -> AppResult<Json<ApiResponse<RoasteryWithLineupsResponse>>> {
    let updated = roastery_studio_service::update_my_roastery(&state, claims.sub, &dto).await?;
    Ok(ApiResponse::ok("Profil Roastery berhasil diperbarui.", updated))
}

pub async fn add_lineup(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<UpsertLineupDto>,
) -> AppResult<(StatusCode, Json<ApiResponse<RoasteryLineupRow>>)> {
    let created = roastery_studio_service::add_lineup(&state, claims.sub, &dto).await?;
    Ok(ApiResponse::created("Varian kopi lineup berhasil ditambahkan.", created))
}

pub async fn update_lineup(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(lineup_id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<UpsertLineupDto>,
) -> AppResult<Json<ApiResponse<RoasteryLineupRow>>> {
    let updated = roastery_studio_service::update_lineup(&state, claims.sub, lineup_id, &dto).await?;
    Ok(ApiResponse::ok("Varian kopi lineup berhasil diperbarui.", updated))
}

pub async fn delete_lineup(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(lineup_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<DeleteLineupOutcome>>> {
    let deleted = roastery_studio_service::delete_lineup(&state, claims.sub, lineup_id).await?;
    Ok(ApiResponse::ok("Varian kopi lineup berhasil dihapus.", deleted))
}

pub async fn issue_roastery_upload_signature(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<RoasteryUploadSigDto>,
) -> AppResult<Json<ApiResponse<CloudinaryUploadSignature>>> {
    let sig = cloudinary_asset_service::sign_roastery_asset_upload(
        &state.cloudinary,
        claims.sub,
        &dto.asset_kind,
    )?;
    Ok(ApiResponse::ok("Signature Cloudinary Direct Upload berhasil dibuat.", sig))
}
