use axum::{extract::State, http::StatusCode, Json};

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{RequireTier, TierBusiness, ValidatedJson},
        AppState,
    },
    modules::live_brew::{
        address_cipher::encode_static_table_code,
        error::LiveBrewError,
        mirror_dto::{
            BrandedSeatResolutionDto, BusinessBrandingDto, CafeSeatDto, CreateSeatBatchRequest,
            CreateSeatRequest, PosOrderBindRequest, UpsertBrandingRequest,
        },
        mirror_models::slugify_livebrew_segment,
        mirror_protocol::CafeSeatKind,
        repositories::branding_seat_repo,
        services::{
            mirror_state_machine::{row_to_branding_dto, row_to_seat_dto},
            pos_order_sync_bridge,
        },
    },
};

pub async fn get_my_branding(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Option<BusinessBrandingDto>>>> {
    let row = branding_seat_repo::fetch_branding_by_owner(&state.pool, claims.sub).await?;
    Ok(ApiResponse::ok(
        "Konfigurasi branding Live Brew Bisnis dimuat.",
        row.as_ref().map(row_to_branding_dto),
    ))
}

pub async fn upsert_my_branding(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(req): ValidatedJson<UpsertBrandingRequest>,
) -> AppResult<Json<ApiResponse<BusinessBrandingDto>>> {
    let row = branding_seat_repo::upsert_branding(&state.pool, claims.sub, &req).await?;
    Ok(ApiResponse::ok(
        "Branding kafe & URL kustom Live Brew berhasil disimpan.",
        row_to_branding_dto(&row),
    ))
}

pub async fn list_my_seats(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Vec<CafeSeatDto>>>> {
    let Some(branding) = branding_seat_repo::fetch_branding_by_owner(&state.pool, claims.sub).await?
    else {
        return Ok(ApiResponse::ok("Belum ada meja/kursi terdaftar.", Vec::new()));
    };
    let rows = branding_seat_repo::list_seats_for_store(&state.pool, branding.store_id).await?;
    let dtos = rows
        .iter()
        .map(|r| row_to_seat_dto(r, &branding.cafe_slug))
        .collect();
    Ok(ApiResponse::ok("Daftar meja & kursi kafe berhasil dimuat.", dtos))
}

pub async fn create_seat(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(req): ValidatedJson<CreateSeatRequest>,
) -> AppResult<(StatusCode, Json<ApiResponse<CafeSeatDto>>)> {
    let branding = ensure_default_branding(&state, claims.sub, &claims.username).await?;
    let slug = slugify_livebrew_segment(
        req.seat_slug.as_deref().unwrap_or(&req.seat_label),
        48,
    );
    let seq_seed: u64 = (chrono::Utc::now().timestamp_micros().unsigned_abs() % 80_000_000_000) + 100;
    let fallback_code = encode_static_table_code(seq_seed, state.config.jwt_secret_str());

    let row = branding_seat_repo::create_or_upsert_seat(
        &state.pool,
        branding.store_id,
        req.seat_kind.unwrap_or(CafeSeatKind::Meja),
        &req.seat_label,
        &slug,
        req.zone_area.as_deref().unwrap_or("Main Area"),
        req.capacity_persons.unwrap_or(2),
        &fallback_code,
    )
    .await?;

    Ok(ApiResponse::created(
        "Barcode meja/kursi kustom berhasil dibuat.",
        row_to_seat_dto(&row, &branding.cafe_slug),
    ))
}

pub async fn create_seat_batch(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(req): ValidatedJson<CreateSeatBatchRequest>,
) -> AppResult<(StatusCode, Json<ApiResponse<Vec<CafeSeatDto>>>)> {
    let start = req.start_number.max(1);
    let end = req.end_number.min(start.saturating_add(49));
    if end < start {
        return Err(LiveBrewError::SeatSlugConflict("Rentang nomor tidak valid".into()).into());
    }
    let branding = ensure_default_branding(&state, claims.sub, &claims.username).await?;
    let kind = req.seat_kind.unwrap_or(CafeSeatKind::Meja);
    let zone = req.zone_area.as_deref().unwrap_or("Main Area");
    let cap = req.capacity_persons.unwrap_or(2);

    let mut created = Vec::new();
    let base_seed = chrono::Utc::now().timestamp_micros().unsigned_abs() % 70_000_000_000;
    for (idx, num) in (start..=end).enumerate() {
        let label = format!("{} {:02}", req.prefix.trim(), num);
        let slug = slugify_livebrew_segment(&label, 48);
        let code = encode_static_table_code(
            base_seed + (idx as u64) + 10,
            state.config.jwt_secret_str(),
        );
        let row = branding_seat_repo::create_or_upsert_seat(
            &state.pool,
            branding.store_id,
            kind,
            &label,
            &slug,
            zone,
            cap,
            &code,
        )
        .await?;
        created.push(row_to_seat_dto(&row, &branding.cafe_slug));
    }

    Ok(ApiResponse::created(
        format!("{} barcode meja/kursi siap cetak berhasil dibuat.", created.len()),
        created,
    ))
}

pub async fn bind_pos_order_to_seat(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(req): ValidatedJson<PosOrderBindRequest>,
) -> AppResult<Json<ApiResponse<BrandedSeatResolutionDto>>> {
    let resolution =
        pos_order_sync_bridge::bind_pos_order_to_seat(&state, claims.sub, &req).await?;
    Ok(ApiResponse::ok(
        "Pesanan POS berhasil disinkronkan secara real-time ke barcode meja.",
        resolution,
    ))
}

async fn ensure_default_branding(
    state: &AppState,
    owner_id: uuid::Uuid,
    username: &str,
) -> Result<crate::modules::live_brew::mirror_models::BusinessBrandingRow, LiveBrewError> {
    if let Some(existing) = branding_seat_repo::fetch_branding_by_owner(&state.pool, owner_id).await? {
        return Ok(existing);
    }
    let slug = slugify_livebrew_segment(&format!("cafe-{}", username), 64);
    let req = UpsertBrandingRequest {
        cafe_slug: slug,
        cafe_display_name: format!("{} Coffee Bar", username),
        logo_cloudinary_url: None,
        logo_cloudinary_public_id: None,
        primary_color_hex: Some("#12100E".to_string()),
        accent_color_hex: Some("#D4A373".to_string()),
        welcome_message: None,
        wifi_ssid: None,
        wifi_password: None,
        instagram_handle: None,
        show_pos_queue_position: Some(true),
        show_live_bean_details: Some(true),
    };
    branding_seat_repo::upsert_branding(&state.pool, owner_id, &req).await
}
