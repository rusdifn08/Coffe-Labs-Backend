use std::time::Duration;

use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::core::{cache::AppCaches, error::AppError};

const FCM_OAUTH_CACHE_KEY: &str = "fcm_oauth_bearer";
const FCM_OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountCreds {
    pub project_id: String,
    pub client_email: String,
    pub private_key: String,
    #[serde(default = "default_token_uri")]
    pub token_uri: String,
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

#[derive(Debug, Serialize)]
struct GoogleJwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResp {
    access_token: String,
}

/// Google Service Account JWT Signer & OAuth2 Bearer Token Manager untuk FCM HTTP v1.
#[derive(Clone)]
pub struct FcmOAuthManager {
    creds: Option<ServiceAccountCreds>,
    http: reqwest::Client,
    project_id_override: Option<String>,
}

impl FcmOAuthManager {
    pub fn from_config(creds_path: Option<&str>, project_id: &str) -> Self {
        let http = reqwest::Client::builder()
            .pool_max_idle_per_host(16)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let creds = Self::load_credentials(creds_path);
        if creds.is_none() {
            warn!(
                "FCM v1 credentials belum dikonfigurasi (GOOGLE_APPLICATION_CREDENTIALS / FIREBASE_SERVICE_ACCOUNT_JSON); gateway berjalan pada mode dry-run lokal."
            );
        }

        let trimmed_pid = project_id.trim();
        let project_id_override = (!trimmed_pid.is_empty()).then(|| trimmed_pid.to_string());

        Self {
            creds,
            http,
            project_id_override,
        }
    }

    pub fn from_env() -> Self {
        let path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let project_id = std::env::var("FCM_PROJECT_ID")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "coffe-labs".to_string());
        Self::from_config(path.as_deref(), &project_id)
    }

    fn load_credentials(creds_path: Option<&str>) -> Option<ServiceAccountCreds> {
        if let Ok(raw_json) = std::env::var("FIREBASE_SERVICE_ACCOUNT_JSON") {
            if !raw_json.trim().is_empty() {
                if let Ok(parsed) = serde_json::from_str::<ServiceAccountCreds>(&raw_json) {
                    return Some(parsed);
                }
            }
        }
        if let Some(path) = creds_path.map(str::trim).filter(|s| !s.is_empty()) {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(parsed) = serde_json::from_str::<ServiceAccountCreds>(&content) {
                    return Some(parsed);
                }
            }
        }
        None
    }

    pub fn is_configured(&self) -> bool {
        self.creds.is_some()
    }

    pub fn creds(&self) -> Option<&ServiceAccountCreds> {
        self.creds.as_ref()
    }

    pub fn resolved_project_id(&self) -> &str {
        if let Some(pid) = &self.project_id_override {
            if pid != "coffe-labs" || self.creds.is_none() {
                return pid.as_str();
            }
        }
        self.creds
            .as_ref()
            .map(|c| c.project_id.as_str())
            .unwrap_or("coffe-labs")
    }

    pub async fn invalidate_cache(&self, caches: &AppCaches) {
        caches.fcm_token_l1.invalidate(FCM_OAUTH_CACHE_KEY).await;
    }

    /// Mengambil OAuth2 Bearer token dari L1 `moka` cache (55m TTL) atau menandatangani
    /// RS256 JWT assertion dan menukarkannya ke `https://oauth2.googleapis.com/token`.
    pub async fn get_valid_bearer_token(&self, caches: &AppCaches) -> Result<String, AppError> {
        if let Some(cached) = caches.fcm_token_l1.get(FCM_OAUTH_CACHE_KEY).await {
            return Ok(cached);
        }
        let creds = self.creds.as_ref().ok_or_else(|| {
            AppError::Unauthorized(
                "FCM service account credentials belum dikonfigurasi (GOOGLE_APPLICATION_CREDENTIALS)."
                    .to_string(),
            )
        })?;

        let token = self.exchange_jwt_for_token(creds).await?;
        caches
            .fcm_token_l1
            .insert(FCM_OAUTH_CACHE_KEY.to_string(), token.clone())
            .await;
        Ok(token)
    }

    async fn exchange_jwt_for_token(&self, creds: &ServiceAccountCreds) -> Result<String, AppError> {
        let now = Utc::now().timestamp();
        let claims = GoogleJwtClaims {
            iss: &creds.client_email,
            scope: FCM_OAUTH_SCOPE,
            aud: &creds.token_uri,
            iat: now,
            exp: now + 3_500,
        };
        let key = EncodingKey::from_rsa_pem(creds.private_key.as_bytes())
            .map_err(|e| AppError::Internal(format!("RSA private key FCM invalid: {e}")))?;
        let assertion = encode(&Header::new(Algorithm::RS256), &claims, &key)
            .map_err(|e| AppError::Internal(format!("Gagal sign JWT assertion FCM: {e}")))?;

        let resp = self
            .http
            .post(&creds.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OAuth2 FCM network error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "OAuth2 token exchange gagal (HTTP {status}): {body}"
            )));
        }

        let parsed: GoogleTokenResp = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Parse respons OAuth2 FCM gagal: {e}")))?;
        Ok(parsed.access_token)
    }
}
