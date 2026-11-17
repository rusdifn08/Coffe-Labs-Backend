use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::communities::dto::{CreateInviteSecretRequest, JoinBySecretRequest},
};

pub async fn join_by_secret(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<JoinBySecretRequest>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .secret_join_service
        .join_by_secret_code(claims.sub, &req.secret_code)
        .await?;
    Ok(ApiResponse::ok("Secret ID valid. Selamat datang di Club!", data))
}

pub async fn join_by_token(
    AuthUser(claims): AuthUser,
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .secret_join_service
        .join_by_invite_token(claims.sub, &token)
        .await?;
    Ok(ApiResponse::ok("Undangan link valid. Selamat datang di Club!", data))
}

pub async fn list_invites(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .secret_join_service
        .list_invites(claims.sub, community_id)
        .await?;
    Ok(ApiResponse::ok("Daftar Secret ID aktif berhasil dimuat.", data))
}

pub async fn create_invite(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<CreateInviteSecretRequest>,
) -> AppResult<impl IntoResponse> {
    let data = state
        .secret_join_service
        .create_invite(claims.sub, community_id, req)
        .await?;
    Ok(ApiResponse::created("Secret ID & link undangan baru berhasil dibuat.", data))
}

pub async fn revoke_invite(
    AuthUser(claims): AuthUser,
    Path((community_id, invite_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let revoked = state
        .secret_join_service
        .revoke_invite(claims.sub, community_id, invite_id)
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
