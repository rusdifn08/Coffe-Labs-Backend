use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{
    core::AppState,
    modules::affiliate::handlers::{
        admin_payout_handler, affiliate_user_handler, withdrawal_user_handler,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/affiliate/me", get(affiliate_user_handler::get_my_affiliate_dashboard))
        .route("/api/v1/affiliate/code", put(affiliate_user_handler::customize_my_affiliate_code))
        .route("/api/v1/affiliate/validate-code", post(affiliate_user_handler::validate_referral_code))
        .route("/api/v1/affiliate/withdrawals", post(withdrawal_user_handler::create_withdrawal_request))
        .route("/api/v1/affiliate/withdrawals", get(withdrawal_user_handler::list_my_withdrawals))
        .route(
            "/api/v1/affiliate/withdrawals/{id}/cancel",
            post(withdrawal_user_handler::cancel_my_withdrawal),
        )
        .route("/api/v1/affiliate/admin/withdrawals", get(admin_payout_handler::list_admin_queue))
        .route(
            "/api/v1/affiliate/admin/payout-proof-signature",
            post(admin_payout_handler::sign_payout_proof),
        )
        .route(
            "/api/v1/affiliate/admin/withdrawals/{id}/processing",
            post(admin_payout_handler::mark_processing),
        )
        .route(
            "/api/v1/affiliate/admin/withdrawals/{id}/approve",
            post(admin_payout_handler::approve_payout),
        )
        .route(
            "/api/v1/affiliate/admin/withdrawals/{id}/reject",
            post(admin_payout_handler::reject_payout),
        )
}
