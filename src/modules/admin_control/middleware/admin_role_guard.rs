use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use uuid::Uuid;

use crate::core::{
    auth_extractor::AuthenticatedUser,
    error::AppError,
    jwt::JwtManager,
};
use crate::modules::admin_control::error::AdminControlError;

/// Extractor RBAC ketat yang menjamin hanya akun ber-role `ADMIN` yang dapat
/// mengakses endpoint mutasi Control Panel (Lock/Unlock Kartu & Dynamic Pricing).
#[derive(Debug, Clone)]
pub struct RequireAdminUser {
    pub user_id: Uuid,
    pub admin_label: String,
}

impl<S> FromRequestParts<S> for RequireAdminUser
where
    JwtManager: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthenticatedUser::from_request_parts(parts, state).await?;
        if !auth.claims.role.eq_ignore_ascii_case("admin") {
            return Err(AdminControlError::AdminRoleRequired.into());
        }
        let label = if auth.claims.username.trim().is_empty() {
            format!("admin:{}", auth.user_id)
        } else {
            auth.claims.username.clone()
        };
        Ok(Self {
            user_id: auth.user_id,
            admin_label: label,
        })
    }
}
