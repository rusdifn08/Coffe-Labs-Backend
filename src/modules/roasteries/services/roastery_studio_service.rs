use uuid::Uuid;

use crate::core::{
    error::{AppError, AppResult},
    AppState,
};
use crate::modules::roasteries::{
    dto::{slugify_roastery_name, CreateRoasteryDto, UpdateRoasteryDto, UpsertLineupDto},
    models::{DeleteLineupOutcome, RoasteryLineupRow, RoasteryWithLineupsResponse},
    repositories::{lineup_repo, roastery_repo, roastery_write_repo},
    services::{
        cloudinary_asset_service::spawn_cleanup_old_asset,
        roastery_catalog_service::{
            assemble_response, optimize_lineup_asset, CATALOG_CACHE_KEY_DEFAULT,
        },
    },
};

pub async fn get_my_roastery(
    state: &AppState,
    owner_user_id: Uuid,
) -> AppResult<Option<RoasteryWithLineupsResponse>> {
    let Some(roastery) = roastery_repo::find_by_owner(&state.pool, owner_user_id).await? else {
        return Ok(None);
    };
    let lineups = lineup_repo::list_for_roastery(&state.pool, roastery.id).await?;
    Ok(Some(assemble_response(roastery, lineups)))
}

pub async fn create_my_roastery(
    state: &AppState,
    owner_user_id: Uuid,
    dto: &CreateRoasteryDto,
) -> AppResult<RoasteryWithLineupsResponse> {
    if roastery_repo::find_by_owner(&state.pool, owner_user_id).await?.is_some() {
        return Err(AppError::Conflict(
            "Setiap akun Business hanya dapat memiliki 1 profil Roastery. Gunakan Kelola Roastery Saya."
                .to_string(),
        ));
    }

    let suffix = &owner_user_id.simple().to_string()[..8];
    let slug = slugify_roastery_name(&dto.name, suffix);
    let created = roastery_write_repo::insert_roastery(&state.pool, owner_user_id, &slug, dto).await?;
    state.caches.directory_l1.invalidate(CATALOG_CACHE_KEY_DEFAULT).await;
    Ok(assemble_response(created, Vec::new()))
}

pub async fn update_my_roastery(
    state: &AppState,
    owner_user_id: Uuid,
    dto: &UpdateRoasteryDto,
) -> AppResult<RoasteryWithLineupsResponse> {
    let previous = roastery_repo::find_by_owner(&state.pool, owner_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Anda belum memiliki profil Roastery terdaftar.".to_string()))?;

    let updated = roastery_write_repo::update_roastery_by_owner(&state.pool, owner_user_id, dto).await?;

    if let (Some(old_pid), Some(new_pid)) = (previous.logo_public_id, dto.logo_public_id.as_deref()) {
        if !old_pid.is_empty() && old_pid != new_pid.trim() {
            spawn_cleanup_old_asset(state.cloudinary.clone(), Some(old_pid));
        }
    }
    if let (Some(old_b_pid), Some(new_b_pid)) = (previous.banner_public_id, dto.banner_public_id.as_deref()) {
        if !old_b_pid.is_empty() && old_b_pid != new_b_pid.trim() {
            spawn_cleanup_old_asset(state.cloudinary.clone(), Some(old_b_pid));
        }
    }

    state.caches.directory_l1.invalidate(CATALOG_CACHE_KEY_DEFAULT).await;
    let lineups = lineup_repo::list_for_roastery(&state.pool, updated.id).await?;
    Ok(assemble_response(updated, lineups))
}

pub async fn add_lineup(
    state: &AppState,
    owner_user_id: Uuid,
    dto: &UpsertLineupDto,
) -> AppResult<RoasteryLineupRow> {
    let roastery = roastery_repo::find_by_owner(&state.pool, owner_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Daftarkan profil Roastery Anda terlebih dahulu sebelum menambah kopi.".to_string()))?;

    let inserted = lineup_repo::insert_lineup(&state.pool, roastery.id, dto).await?;
    roastery_write_repo::sync_lineups_count(&state.pool, roastery.id).await?;
    state.caches.directory_l1.invalidate(CATALOG_CACHE_KEY_DEFAULT).await;
    Ok(optimize_lineup_asset(inserted))
}

pub async fn update_lineup(
    state: &AppState,
    owner_user_id: Uuid,
    lineup_id: Uuid,
    dto: &UpsertLineupDto,
) -> AppResult<RoasteryLineupRow> {
    let roastery = roastery_repo::find_by_owner(&state.pool, owner_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Profil Roastery Anda belum terdaftar.".to_string()))?;

    let previous = lineup_repo::find_owned_lineup(&state.pool, lineup_id, roastery.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Varian kopi lineup tidak ditemukan atau bukan milik Anda.".to_string()))?;

    let updated = lineup_repo::update_lineup(&state.pool, lineup_id, roastery.id, dto).await?;

    if let (Some(old_pid), Some(new_pid)) = (previous.bag_image_public_id, dto.bag_image_public_id.as_deref()) {
        if !old_pid.is_empty() && old_pid != new_pid.trim() {
            spawn_cleanup_old_asset(state.cloudinary.clone(), Some(old_pid));
        }
    }

    state.caches.directory_l1.invalidate(CATALOG_CACHE_KEY_DEFAULT).await;
    Ok(optimize_lineup_asset(updated))
}

pub async fn delete_lineup(
    state: &AppState,
    owner_user_id: Uuid,
    lineup_id: Uuid,
) -> AppResult<DeleteLineupOutcome> {
    let roastery = roastery_repo::find_by_owner(&state.pool, owner_user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Profil Roastery Anda belum terdaftar.".to_string()))?;

    let deleted = lineup_repo::delete_lineup(&state.pool, lineup_id, roastery.id).await?;
    let remaining = roastery_write_repo::sync_lineups_count(&state.pool, roastery.id).await?;

    spawn_cleanup_old_asset(state.cloudinary.clone(), deleted.bag_image_public_id);
    state.caches.directory_l1.invalidate(CATALOG_CACHE_KEY_DEFAULT).await;

    Ok(DeleteLineupOutcome {
        deleted_id: deleted.id,
        roastery_id: roastery.id,
        remaining_lineups_count: remaining,
    })
}
