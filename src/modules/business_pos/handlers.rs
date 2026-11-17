use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use uuid::Uuid;

use crate::core::{
    error::{ApiResponse, AppError, AppResult},
    extractors::{RequireTier, TierBusiness, ValidatedJson},
    AppState,
};
use crate::modules::business_pos::{
    dto::{CreateMenuItemDto, CreatePosOrderDto, UpdateBusinessConfigDto, UpdateMenuItemDto, VerifyPinDto, VerifyPinResponse},
    models::{BusinessMenuItem, BusinessStoreConfig, PosOrder},
    services::{AssetTransparencyValidator, BusinessPosService},
};

pub async fn get_config(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<BusinessStoreConfig>>> {
    let config = BusinessPosService::get_config(&state.pool, claims.sub).await?;
    Ok(ApiResponse::ok("Konfigurasi Business Store berhasil diambil.", config))
}

pub async fn update_config(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<UpdateBusinessConfigDto>,
) -> AppResult<Json<ApiResponse<BusinessStoreConfig>>> {
    let config = BusinessPosService::update_config(&state.pool, claims.sub, dto).await?;
    Ok(ApiResponse::ok("Pengaturan Mode Kasir & AOD Display berhasil disimpan.", config))
}

pub async fn list_menu(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
) -> AppResult<Json<ApiResponse<Vec<BusinessMenuItem>>>> {
    let items = BusinessPosService::list_menu(&state.pool, claims.sub).await?;
    Ok(ApiResponse::ok("Katalog menu berhasil dimuat.", items))
}

pub async fn create_menu_item(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<CreateMenuItemDto>,
) -> AppResult<Json<ApiResponse<BusinessMenuItem>>> {
    let item = BusinessPosService::add_menu_item(&state.pool, claims.sub, dto).await?;
    Ok(ApiResponse::ok("Menu baru berhasil ditambahkan.", item))
}

pub async fn update_menu_item(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(item_id): Path<Uuid>,
    ValidatedJson(dto): ValidatedJson<UpdateMenuItemDto>,
) -> AppResult<Json<ApiResponse<Option<BusinessMenuItem>>>> {
    let item = BusinessPosService::update_menu_item(&state.pool, claims.sub, item_id, dto).await?;
    Ok(ApiResponse::ok("Menu berhasil diperbarui.", item))
}

pub async fn delete_menu_item(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    Path(item_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<bool>>> {
    let success = BusinessPosService::delete_menu_item(&state.pool, claims.sub, item_id).await?;
    Ok(ApiResponse::ok("Menu berhasil dihapus.", success))
}

pub async fn create_pos_order(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<CreatePosOrderDto>,
) -> AppResult<Json<ApiResponse<PosOrder>>> {
    let order = BusinessPosService::create_pos_order(&state.pool, claims.sub, dto).await?;
    Ok(ApiResponse::ok("Transaksi kasir berhasil dicatat.", order))
}

pub async fn verify_pin(
    State(state): State<AppState>,
    RequireTier(claims, _): RequireTier<TierBusiness>,
    ValidatedJson(dto): ValidatedJson<VerifyPinDto>,
) -> AppResult<Json<ApiResponse<VerifyPinResponse>>> {
    let user_id = claims.sub;
    let _ = BusinessPosService::verify_pin(&state.pool, user_id, &dto.pin).await?;
    let config = BusinessPosService::get_config(&state.pool, user_id).await?;
    Ok(ApiResponse::ok(
        "PIN Kasir terverifikasi.",
        VerifyPinResponse {
            verified: true,
            store_name: config.store_name,
        },
    ))
}

pub async fn upload_validate_asset(
    State(_state): State<AppState>,
    RequireTier(_claims, _): RequireTier<TierBusiness>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart upload error: {}", e)))?
    {
        let filename = field
            .file_name()
            .unwrap_or("menu_asset.png")
            .to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        let bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Gagal membaca byte file: {}", e)))?;

        let format = AssetTransparencyValidator::validate_asset_bytes(&filename, &content_type, &bytes)?;

        let base64_str = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
        let mime = if format == "SVG" { "image/svg+xml" } else if format == "WEBP" { "image/webp" } else { "image/png" };
        let data_url = format!("data:{};base64,{}", mime, base64_str);

        return Ok(ApiResponse::ok(
            "Validasi transparansi berhasil.",
            serde_json::json!({
                "format": format,
                "asset_url": data_url,
                "filename": filename,
                "bytes_len": bytes.len(),
                "has_transparency": true
            }),
        ));
    }

    Err(AppError::BadRequest("Tidak ada file gambar yang dikirim.".to_string()))
}
