use axum::{
    extract::{rejection::JsonRejection, FromRef, FromRequest, FromRequestParts, Request},
    http::request::Parts,
    Json,
};
use serde::de::DeserializeOwned;
use sqlx::PgPool;
use std::marker::PhantomData;
use validator::Validate;

use crate::core::{
    auth_extractor::{AuthenticatedUser, OptionalAuthenticatedUser},
    error::AppError,
    jwt::{JwtClaims, JwtManager},
};

pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("Format payload JSON tidak valid: {}", e.body_text())))?;

        value.validate().map_err(|errs| {
            let messages: Vec<String> = errs
                .field_errors()
                .iter()
                .flat_map(|(field, errors)| {
                    errors.iter().map(move |err| {
                        err.message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| format!("Field '{}' tidak memenuhi aturan validasi", field))
                    })
                })
                .collect();
            let msg = if messages.is_empty() {
                "Validasi input gagal.".to_string()
            } else {
                messages.join("; ")
            };
            AppError::Validation(msg)
        })?;

        Ok(ValidatedJson(value))
    }
}

#[derive(Debug, Clone)]
pub struct AuthUser(pub JwtClaims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    JwtManager: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;
        Ok(AuthUser(user.claims))
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AdminUser(pub JwtClaims);

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    JwtManager: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        if claims.role != "admin" {
            return Err(AppError::Forbidden(
                "Akses terlarang: Hanya akun Admin yang berhak mengakses fitur ini.".to_string(),
            ));
        }
        Ok(AdminUser(claims))
    }
}

#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<JwtClaims>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
    JwtManager: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let OptionalAuthenticatedUser(opt_user) =
            OptionalAuthenticatedUser::from_request_parts(parts, state).await?;
        Ok(OptionalAuthUser(opt_user.map(|u| u.claims)))
    }
}

pub trait TierRequirement: Send + Sync + 'static {
    fn required_level() -> u8;
    fn tier_name() -> &'static str;
}

#[allow(dead_code)]
pub struct TierTrial;
pub struct TierPro;
pub struct TierBusiness;

impl TierRequirement for TierTrial {
    fn required_level() -> u8 { 1 }
    fn tier_name() -> &'static str { "Trial" }
}
impl TierRequirement for TierPro {
    fn required_level() -> u8 { 2 }
    fn tier_name() -> &'static str { "Pro" }
}
impl TierRequirement for TierBusiness {
    fn required_level() -> u8 { 3 }
    fn tier_name() -> &'static str { "Business" }
}

pub struct RequireTier<T: TierRequirement>(pub JwtClaims, pub PhantomData<T>);

impl<S, T> FromRequestParts<S> for RequireTier<T>
where
    S: Send + Sync,
    JwtManager: FromRef<S>,
    PgPool: FromRef<S>,
    T: TierRequirement,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUser(mut claims) = AuthUser::from_request_parts(parts, state).await?;
        let pool = PgPool::from_ref(state);
        if let Ok(Some((live_tier, live_role))) = sqlx::query_as::<_, (String, String)>(
            "SELECT tier, role FROM users WHERE id = $1",
        )
        .bind(claims.sub)
        .fetch_optional(&pool)
        .await
        {
            claims.tier = live_tier;
            claims.role = live_role;
        }
        let user_level = if claims.role.eq_ignore_ascii_case("admin") {
            3
        } else {
            match claims.tier.trim().to_lowercase().as_str() {
                "business" => 3,
                "pro" => 2,
                _ => 1,
            }
        };

        if user_level < T::required_level() {
            return Err(AppError::Forbidden(format!(
                "Fitur ini memerlukan lisensi Coffee Labs {} atau lebih tinggi.",
                T::tier_name()
            )));
        }
        Ok(RequireTier(claims, PhantomData))
    }
}
