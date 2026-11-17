use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use uuid::Uuid;

use crate::core::{
    error::AppError,
    jwt::{JwtClaims, JwtManager},
};

/// Identitas pengguna terautentikasi dari header `Authorization: Bearer` atau query `?token=`.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    #[allow(dead_code)]
    pub account_tier: String,
    pub claims: JwtClaims,
}

/// Mengekstrak token JWT mentah dengan prioritas:
/// 1. Header HTTP `Authorization: Bearer <token>` (REST API & Mobile `IOWebSocketChannel`)
/// 2. Parameter query URI `?token=<jwt>` (WebSocket Web / Fallback)
pub fn extract_raw_jwt(parts: &Parts) -> Option<String> {
    if let Some(raw_header) = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some((scheme, rest)) = raw_header.trim().split_once(' ') {
            if scheme.eq_ignore_ascii_case("bearer") {
                let candidate = rest.trim();
                if !candidate.is_empty() {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    let query = parts.uri.query()?;
    for pair in query.split('&') {
        if let Some((key, val)) = pair.split_once('=') {
            if key == "token" {
                let decoded = percent_decode_query_component(val);
                let candidate = decoded.trim();
                if !candidate.is_empty() {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    None
}

fn percent_decode_query_component(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (hex_nibble(bytes[i + 1]), hex_nibble(bytes[i + 2])) {
                    out.push((hi << 4) | lo);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    JwtManager: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let raw_token = extract_raw_jwt(parts).ok_or_else(|| {
            AppError::Unauthorized(
                "Token autentikasi diperlukan (Authorization: Bearer atau ?token=).".to_string(),
            )
        })?;

        let jwt = JwtManager::from_ref(state);
        let claims = jwt.verify_token(&raw_token)?;
        if claims.status.eq_ignore_ascii_case("declined") {
            return Err(AppError::Unauthorized(
                "Akun Anda dinonaktifkan dan tidak lagi memiliki akses.".to_string(),
            ));
        }

        Ok(Self {
            user_id: claims.sub,
            account_tier: claims.tier.clone(),
            claims,
        })
    }
}

/// Extractor opsional yang tidak melempar 401 apabila klien belum login atau token tidak valid.
#[derive(Debug, Clone)]
pub struct OptionalAuthenticatedUser(pub Option<AuthenticatedUser>);

impl<S> FromRequestParts<S> for OptionalAuthenticatedUser
where
    JwtManager: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Some(raw_token) = extract_raw_jwt(parts) else {
            return Ok(Self(None));
        };

        let jwt = JwtManager::from_ref(state);
        match jwt.verify_token(&raw_token) {
            Ok(claims) if !claims.status.eq_ignore_ascii_case("declined") => {
                Ok(Self(Some(AuthenticatedUser {
                    user_id: claims.sub,
                    account_tier: claims.tier.clone(),
                    claims,
                })))
            }
            _ => Ok(Self(None)),
        }
    }
}
