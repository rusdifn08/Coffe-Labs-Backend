use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::notifications::{
        dto::{InboxQuery, UpdatePreferencesReq},
        services::inbox_service,
    },
};

fn parse_uid(sub: &str) -> AppResult<Uuid> {
    Uuid::parse_str(sub).map_err(|_| AppError::Unauthorized("Identitas JWT tidak valid.".to_string()))
}

pub async fn get_inbox(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Query(query): Query<InboxQuery>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let page = inbox_service::fetch_inbox_page(
        &state.pool,
        user_id,
        query.category.as_deref(),
        query.cursor.as_deref(),
        query.limit,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok("Daftar kotak masuk notifikasi berhasil dimuat.", page))
}

pub async fn get_unread_count(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let unread_count = inbox_service::get_unread_count(&state.pool, user_id)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok(
        "Jumlah notifikasi belum dibaca diperbarui.",
        json!({ "unread_count": unread_count }),
    ))
}

pub async fn post_mark_read(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(notif_id): Path<Uuid>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let unread_count = inbox_service::mark_read(&state.pool, user_id, notif_id)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok(
        "Notifikasi ditandai sudah dibaca.",
        json!({ "unread_count": unread_count }),
    ))
}

pub async fn post_mark_all_read(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let unread_count = inbox_service::mark_all_read(&state.pool, user_id)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok(
        "Seluruh notifikasi ditandai sudah dibaca.",
        json!({ "unread_count": unread_count }),
    ))
}

pub async fn get_preferences(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let prefs = inbox_service::get_preferences(&state.pool, user_id)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Preferensi notifikasi berhasil dimuat.", prefs))
}

pub async fn put_preferences(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<UpdatePreferencesReq>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let prefs = inbox_service::update_preferences(&state.pool, user_id, &req)
        .await
        .map_err(AppError::from)?;
    Ok(ApiResponse::ok("Preferensi notifikasi berhasil disimpan.", prefs))
}
