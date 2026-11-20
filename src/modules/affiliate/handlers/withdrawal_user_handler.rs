use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    core::{
        auth_extractor::AuthenticatedUser,
        error::{ApiResponse, AppResult},
        AppState,
    },
    modules::affiliate::{
        dto::{AffiliateWithdrawalDto, CreateWithdrawalDto},
        repositories::withdrawal_repo,
        services::{withdrawal_admin_service, withdrawal_payout_service},
    },
};

/// POST /api/v1/affiliate/withdrawals — Ajukan penarikan saldo minimal Rp 100.000 (1x24 jam kerja).
pub async fn create_withdrawal_request(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Json(req): Json<CreateWithdrawalDto>,
) -> AppResult<Json<ApiResponse<AffiliateWithdrawalDto>>> {
    let created = withdrawal_payout_service::request_user_withdrawal(&state, auth.user_id, req).await?;
    Ok(ApiResponse::ok(
        "Permintaan penarikan berhasil dibuat! Dana akan ditransfer manual oleh Admin maks. 1x24 jam hari kerja.",
        AffiliateWithdrawalDto::from(created),
    ))
}

/// GET /api/v1/affiliate/withdrawals — Riwayat penarikan saldo milik afiliator.
pub async fn list_my_withdrawals(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<AffiliateWithdrawalDto>>>> {
    let rows = withdrawal_repo::list_user_withdrawals(&state.pool, auth.user_id, 50).await?;
    let items = rows.into_iter().map(AffiliateWithdrawalDto::from).collect();
    Ok(ApiResponse::ok("Riwayat penarikan saldo berhasil dimuat.", items))
}

/// POST /api/v1/affiliate/withdrawals/{id}/cancel — Batalkan penarikan selama status masih PENDING.
pub async fn cancel_my_withdrawal(
    auth: AuthenticatedUser,
    State(state): State<AppState>,
    Path(withdrawal_id): Path<Uuid>,
) -> AppResult<Json<ApiResponse<AffiliateWithdrawalDto>>> {
    let cancelled = withdrawal_admin_service::reject_or_cancel_withdrawal(
        &state,
        withdrawal_id,
        auth.user_id,
        false,
        None,
    )
    .await?;
    Ok(ApiResponse::ok(
        "Penarikan dibatalkan dan saldo dikembalikan utuh ke Dompet Afiliasi Anda.",
        AffiliateWithdrawalDto::from(cancelled),
    ))
}
