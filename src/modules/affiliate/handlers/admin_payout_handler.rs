use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::{
    core::{
        auth_extractor::AuthenticatedUser,
        error::{ApiResponse, AppError, AppResult},
        storage::CloudinaryUploadSignature,
        AppState,
    },
    modules::affiliate::{
        dto::{
            AdminApproveWithdrawalDto, AdminQueueQueryDto, AdminRejectWithdrawalDto,
            AdminWithdrawalItemDto, AffiliateWithdrawalDto, PayoutProofSignatureReqDto,
        },
        repositories::withdrawal_repo,
        services::{payout_proof_signer, withdrawal_admin_service},
    },
};

fn ensure_admin(auth: &AuthenticatedUser) -> AppResult<()> {
    if !auth.claims.role.eq_ignore_ascii_case("admin") {
        return Err(AppError::Forbidden(
            "Akses ditolak: panel ACC penarikan afiliasi khusus Owner / Admin Coffee Labs.".into(),
        ));
    }
    Ok(())
}

/// GET /api/v1/affiliate/admin/withdrawals — Daftar antrean penarikan saldo untuk ACC Owner/Admin.
pub async fn list_admin_queue(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Query(q): Query<AdminQueueQueryDto>,
) -> AppResult<Json<ApiResponse<Vec<AdminWithdrawalItemDto>>>> {
    ensure_admin(&auth)?;
    let status = q
        .status
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("ALL"));
    let rows =
        withdrawal_repo::list_admin_withdrawal_queue(&state.pool, status, q.limit.unwrap_or(100))
            .await?;
    let items = rows.into_iter().map(AdminWithdrawalItemDto::from).collect();
    Ok(ApiResponse::ok("Antrean penarikan afiliasi admin dimuat.", items))
}

/// POST /api/v1/affiliate/admin/payout-proof-signature — Signature Cloudinary bukti transfer.
pub async fn sign_payout_proof(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<PayoutProofSignatureReqDto>,
) -> AppResult<Json<ApiResponse<CloudinaryUploadSignature>>> {
    ensure_admin(&auth)?;
    let sig = payout_proof_signer::sign_payout_proof_upload(&state.cloudinary, req.withdrawal_id)?;
    Ok(ApiResponse::ok("Signature upload bukti transfer Cloudinary siap.", sig))
}

/// POST /api/v1/affiliate/admin/withdrawals/{id}/processing — Tandai sedang diproses (kunci anti transfer dobel).
pub async fn mark_processing(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(withdrawal_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<AffiliateWithdrawalDto>>> {
    ensure_admin(&auth)?;
    let updated =
        withdrawal_admin_service::mark_withdrawal_processing(&state, withdrawal_id, auth.user_id)
            .await?;
    Ok(ApiResponse::ok(
        "Tiket penarikan dikunci sebagai Sedang Diproses.",
        AffiliateWithdrawalDto::from(updated),
    ))
}

/// POST /api/v1/affiliate/admin/withdrawals/{id}/approve — ACC & simpan bukti transfer Cloudinary.
pub async fn approve_payout(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(withdrawal_id): Path<Uuid>,
    Json(req): Json<AdminApproveWithdrawalDto>,
) -> AppResult<Json<ApiResponse<AffiliateWithdrawalDto>>> {
    ensure_admin(&auth)?;
    let settled =
        withdrawal_admin_service::approve_withdrawal(&state, withdrawal_id, auth.user_id, req)
            .await?;
    Ok(ApiResponse::ok(
        "Penarikan berhasil di-ACC & bukti transfer telah dikirim ke afiliator.",
        AffiliateWithdrawalDto::from(settled),
    ))
}

/// POST /api/v1/affiliate/admin/withdrawals/{id}/reject — Tolak & kembalikan saldo utuh ke afiliator.
pub async fn reject_payout(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(withdrawal_id): Path<Uuid>,
    Json(req): Json<AdminRejectWithdrawalDto>,
) -> AppResult<Json<ApiResponse<AffiliateWithdrawalDto>>> {
    ensure_admin(&auth)?;
    let refunded = withdrawal_admin_service::reject_or_cancel_withdrawal(
        &state,
        withdrawal_id,
        auth.user_id,
        true,
        Some(req),
    )
    .await?;
    Ok(ApiResponse::ok(
        "Penarikan ditolak dan saldo afiliator dikembalikan penuh.",
        AffiliateWithdrawalDto::from(refunded),
    ))
}
