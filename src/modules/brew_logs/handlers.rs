use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use sqlx::PgPool;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::OptionalAuthUser,
    },
    modules::brew_logs::{dto::BrewLogQuery, service},
};

pub async fn get_brew_history(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<BrewLogQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let list = service::fetch_brew_logs(&pool, auth, "history", query).await?;
    Ok(Json(list))
}

pub async fn save_brew_history(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<impl IntoResponse> {
    let saved = service::record_brew_entry(&pool, auth, "history", payload).await?;
    Ok(ApiResponse::created("Riwayat seduh berhasil dicatat.", saved))
}

pub async fn get_journal_entries(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<BrewLogQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let list = service::fetch_brew_logs(&pool, auth, "journal", query).await?;
    Ok(Json(list))
}

pub async fn save_journal_entry(
    OptionalAuthUser(auth): OptionalAuthUser,
    State(pool): State<PgPool>,
    Json(payload): Json<serde_json::Value>,
) -> AppResult<impl IntoResponse> {
    let saved = service::record_brew_entry(&pool, auth, "journal", payload).await?;
    Ok(ApiResponse::created("Jurnal evaluasi sensori berhasil disimpan.", saved))
}

pub async fn delete_journal_entry(
    OptionalAuthUser(auth): OptionalAuthUser,
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    service::remove_journal_entry(&pool, auth, id).await?;
    Ok(Json(json!({
        "success": true,
        "message": "Catatan jurnal seduh berhasil dihapus."
    })))
}

pub async fn get_brew_analytics(
    OptionalAuthUser(auth): OptionalAuthUser,
    Query(query): Query<BrewLogQuery>,
    State(pool): State<PgPool>,
) -> AppResult<impl IntoResponse> {
    let analytics = service::compute_analytics(&pool, auth, query).await?;
    Ok(ApiResponse::ok("Analitik seduh barista berhasil dimuat.", analytics))
}
