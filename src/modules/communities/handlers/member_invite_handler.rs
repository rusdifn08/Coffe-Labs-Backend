use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    core::{
        error::{ApiResponse, AppResult},
        extractors::{AuthUser, ValidatedJson},
        AppState,
    },
    modules::communities::{
        invite_dto::InviteUsersToClubRequest,
        services::{member_invite_response_service, member_invite_service},
    },
};

pub async fn list_my_pending_invitations(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = member_invite_service::list_my_pending_invitations(&state.pool, claims.sub).await?;
    Ok(ApiResponse::ok("Daftar undangan masuk Coffee Club berhasil dimuat.", data))
}

pub async fn accept_invitation(
    AuthUser(claims): AuthUser,
    Path(invite_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = member_invite_response_service::accept_invitation(
        &state.pool,
        &state.club_cache,
        &state.notifications,
        invite_id,
        claims.sub,
    )
    .await?;
    Ok(ApiResponse::ok("Undangan diterima! Anda resmi bergabung ke Coffee Club.", data))
}

pub async fn decline_invitation(
    AuthUser(claims): AuthUser,
    Path(invite_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = member_invite_response_service::decline_invitation(
        &state.pool,
        invite_id,
        claims.sub,
    )
    .await?;
    Ok(ApiResponse::ok("Undangan masuk Coffee Club telah ditolak.", data))
}

pub async fn send_club_invitations(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<InviteUsersToClubRequest>,
) -> AppResult<impl IntoResponse> {
    let data = member_invite_service::send_direct_invitations(
        &state.pool,
        &state.club_cache,
        &state.notifications,
        claims.sub,
        community_id,
        &req.user_ids,
        req.role_granted.as_deref(),
        req.message.as_deref(),
    )
    .await?;
    Ok(ApiResponse::created("Undangan anggota Coffee Club berhasil dikirim.", data))
}

pub async fn list_club_sent_invitations(
    AuthUser(claims): AuthUser,
    Path(community_id): Path<Uuid>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let data = member_invite_service::list_club_sent_invitations(
        &state.pool,
        claims.sub,
        community_id,
    )
    .await?;
    Ok(ApiResponse::ok("Daftar undangan pending Coffee Club berhasil dimuat.", data))
}

pub async fn cancel_club_invitation(
    AuthUser(claims): AuthUser,
    Path((community_id, invite_id)): Path<(Uuid, Uuid)>,
    State(state): State<AppState>,
) -> AppResult<impl IntoResponse> {
    let cancelled = member_invite_response_service::cancel_invitation(
        &state.pool,
        claims.sub,
        community_id,
        invite_id,
    )
    .await?;
    Ok(ApiResponse::ok(
        if cancelled {
            "Undangan Coffee Club berhasil dibatalkan."
        } else {
            "Undangan tidak ditemukan atau sudah direspon."
        },
        json!({ "cancelled": cancelled, "invitation_id": invite_id }),
    ))
}
