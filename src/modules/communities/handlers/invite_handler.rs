use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppError, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::communities::dto::{CreateInviteSecretRequest, JoinBySecretRequest},
};

fn parse_uid(raw: &str) -> AppResult<Uuid> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest("User ID tidak valid.".into()))
}

pub async fn join_by_secret(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<JoinBySecretRequest>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let data = state
        .secret_join_service
        .join_by_secret_code(user_id, &req.secret_code)
        .await?;
    Ok(ApiResponse::ok("Secret ID valid. Selamat datang di Club!", data))
}

pub async fn join_by_token(
    AuthUser(claims): AuthUser,
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let user_id = parse_uid(&claims.sub)?;
    let data = state
        .secret_join_service
        .join_by_invite_token(user_id, &token)
        .await?;
    Ok(ApiResponse::ok("Undangan link valid. Selamat datang di Club!", data))
}

pub async fn list_invites(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let actor_id = parse_uid(&claims.sub)?;
    let data = state
        .secret_join_service
        .list_invites(actor_id, community_id)
        .await?;
    Ok(ApiResponse::ok("Daftar Secret ID aktif berhasil dimuat.", data))
}

pub async fn create_invite(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<CreateInviteSecretRequest>,
) -> AppResult<impl IntoResponse> {
    let actor_id = parse_uid(&claims.sub)?;
    let data = state
        .secret_join_service
        .create_invite(actor_id, community_id, req)
        .await?;
    Ok(ApiResponse::created("Secret ID & link undangan baru berhasil dibuat.", data))
}

pub async fn revoke_invite(
    AuthUser(claims): AuthUser,
    Path((community_id, invite_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let actor_id = parse_uid(&claims.sub)?;
    let revoked = state
        .secret_join_service
        .revoke_invite(actor_id, community_id, invite_id)
        .await?;
    Ok(ApiResponse::ok(
        if revoked {
            "Secret ID berhasil dicabut."
        } else {
            "Secret ID tidak ditemukan atau sudah dicabut."
        },
        json!({ "revoked": revoked, "invite_id": invite_id }),
    ))
}
