use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::live_brew::{
        address_cipher::landing_livebrew_url,
        dto::{
            BindTableSessionRequest, BrewTableDto, CreateBrewTableRequest,
            CreateLiveBrewSessionRequest, LiveBrewSnapshotDto,
        },
        error::LiveBrewError,
        models::BusinessBrewTableRow,
        repositories::{brew_table_repo, live_session_repo},
        services::{passport_archiver_service::session_row_to_snapshot, table_routing_service},
    },
};

fn table_to_dto(t: &BusinessBrewTableRow) -> BrewTableDto {
    BrewTableDto {
        id: t.id,
        table_number: t.table_number.clone(),
        static_address_code: t.static_address_code.clone(),
        livebrew_url: landing_livebrew_url(&t.static_address_code),
        deep_link_url: format!("coffeelabs://livebrew/{}", t.static_address_code),
        active_brew_session_id: t.active_brew_session_id,
        qr_scans_count: t.qr_scans_count,
        is_active: t.is_active,
    }
}

pub async fn create_live_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<CreateLiveBrewSessionRequest>,
) -> AppResult<impl IntoResponse> {
    let barista_name = if claims.username.trim().is_empty() {
        "Barista Coffee Labs".to_string()
    } else {
        claims.username.clone()
    };
    let secret = state.config.jwt_secret_str();
    let row = live_session_repo::create_session(&state.pool, claims.sub, &barista_name, &req, secret)
        .await
        .map_err(AppError::from)?;

    let mut tbl_num = None;
    let mut tbl_code = None;
    if let Some(tid) = req.table_id {
        if let Some(t) = brew_table_repo::find_table_by_id(&state.pool, tid)
            .await
            .map_err(AppError::from)?
        {
            tbl_num = Some(t.table_number);
            tbl_code = Some(t.static_address_code);
            let _ = table_routing_service::bind_table_and_notify(
                &state.pool,
                &state.live_brew_hub,
                tid,
                row.id,
            )
            .await;
        }
    }

    let (_, store_name) = brew_table_repo::fetch_user_store_info(&state.pool, claims.sub).await;
    let snap: LiveBrewSnapshotDto =
        session_row_to_snapshot(&row, &store_name, tbl_num, tbl_code, Vec::new());

    Ok(ApiResponse::created("Sesi Live Brew berhasil dibuat.", snap))
}

pub async fn list_tables(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<impl IntoResponse> {
    let tables = brew_table_repo::list_owner_tables(&state.pool, claims.sub)
        .await
        .map_err(AppError::from)?;
    let out: Vec<BrewTableDto> = tables.iter().map(table_to_dto).collect();
    Ok(ApiResponse::ok("Daftar meja static QR berhasil dimuat.", out))
}

pub async fn create_table(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    ValidatedJson(req): ValidatedJson<CreateBrewTableRequest>,
) -> AppResult<impl IntoResponse> {
    if req.table_number.trim().is_empty() {
        return Err(AppError::BadRequest("Nomor/nama meja tidak boleh kosong.".to_string()));
    }
    let table = brew_table_repo::create_or_get_table(
        &state.pool,
        claims.sub,
        &req.table_number,
        state.config.jwt_secret_str(),
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::created(
        "Meja static QR berhasil terdaftar.",
        table_to_dto(&table),
    ))
}

pub async fn bind_table_session(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(table_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<BindTableSessionRequest>,
) -> AppResult<impl IntoResponse> {
    let table = brew_table_repo::find_table_by_id(&state.pool, table_id)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::from(LiveBrewError::TableNotFound(table_id.to_string())))?;

    if table.owner_user_id != claims.sub && !claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::from(LiveBrewError::UnauthorizedBarista));
    }

    table_routing_service::bind_table_and_notify(
        &state.pool,
        &state.live_brew_hub,
        table_id,
        req.session_id,
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok("Meja statis berhasil terhubung ke sesi seduh.", table_id))
}

pub async fn resolve_live_address(
    State(state): State<AppState>,
    Path(address_code): Path<String>,
) -> AppResult<impl IntoResponse> {
    let res = table_routing_service::resolve_address_code(
        &state.pool,
        &state.live_brew_hub,
        &address_code,
        state.config.jwt_secret_str(),
    )
    .await
    .map_err(AppError::from)?;

    Ok(ApiResponse::ok("Resolusi alamat Live Brew berhasil.", res))
}
