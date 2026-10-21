use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde_json::json;

use crate::models::Claims;

pub const JWT_SECRET: &[u8] = b"coffee_labs_super_secret_jwt_key_2026";

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

pub fn generate_jwt(user_id: &str, username: &str, role: &str, status: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(30))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        status: status.to_string(),
        exp: expiration,
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET))
}

pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    decode::<Claims>(token, &DecodingKey::from_secret(JWT_SECRET), &validation).map(|data| data.claims)
}

/// Extractor for authenticated user
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        let token = match auth_header {
            Some(auth) if auth.starts_with("Bearer ") => &auth[7..],
            _ => {
                let body = Json(json!({
                    "success": false,
                    "message": "Akses ditolak: Token autentikasi tidak ditemukan."
                }));
                return Err((StatusCode::UNAUTHORIZED, body).into_response());
            }
        };

        match decode_jwt(token) {
            Ok(claims) => Ok(AuthUser(claims)),
            Err(_) => {
                let body = Json(json!({
                    "success": false,
                    "message": "Token tidak valid atau telah kedaluwarsa. Silakan login kembali."
                }));
                Err((StatusCode::UNAUTHORIZED, body).into_response())
            }
        }
    }
}

/// Extractor specifically requiring admin role
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AdminUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;

        if claims.role != "admin" {
            let body = Json(json!({
                "success": false,
                "message": "Akses terlarang: Hanya akun Admin yang berhak mengakses fitur ini."
            }));
            return Err((StatusCode::FORBIDDEN, body).into_response());
        }

        Ok(AdminUser(claims))
    }
}

/// Extractor opsional (menerima request baik ber-token maupun anonim/guest)
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Claims>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());

        let claims = match auth_header {
            Some(auth) if auth.starts_with("Bearer ") => {
                let token = &auth[7..];
                decode_jwt(token).ok()
            }
            _ => None,
        };

        Ok(OptionalAuthUser(claims))
    }
}
