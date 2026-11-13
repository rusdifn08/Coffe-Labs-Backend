use std::time::Duration;

use reqwest::header::RETRY_AFTER;

use crate::{
    core::{cache::AppCaches, fcm_oauth::FcmOAuthManager},
    modules::notifications::{
        error::{http_status_is_retryable, NotifError},
        events::OmnichannelNotifEvent,
        services::{fcm_auth::FcmAuthManager, fcm_payload::build_fcm_v1_message},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FcmDeliveryOutcome {
    Sent,
    UnconfiguredDryRun,
    PruneStaleToken,
    Retryable {
        retry_after_secs: Option<u64>,
        reason: String,
    },
    PermanentReject {
        status: u16,
        body: String,
    },
}

pub struct FcmV1Gateway {
    client: reqwest::Client,
    auth: FcmAuthManager,
}

impl FcmV1Gateway {
    #[allow(dead_code)]
    pub fn from_env() -> Self {
        Self::from_core(FcmOAuthManager::from_env(), AppCaches::new())
    }

    pub fn from_core(oauth: FcmOAuthManager, caches: AppCaches) -> Self {
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(32)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .timeout(Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            auth: FcmAuthManager::new(oauth, caches),
        }
    }

    #[allow(dead_code)]
    pub fn is_configured(&self) -> bool {
        self.auth.is_configured()
    }

    pub async fn send_to_token(
        &self,
        device_token: &str,
        event: &OmnichannelNotifEvent,
    ) -> FcmDeliveryOutcome {
        if !self.auth.is_configured() {
            return FcmDeliveryOutcome::UnconfiguredDryRun;
        }

        let bearer = match self.auth.get_bearer_token().await {
            Ok(tok) => tok,
            Err(NotifError::RetryableDelivery {
                retry_after_secs,
                reason,
            }) => {
                return FcmDeliveryOutcome::Retryable {
                    retry_after_secs,
                    reason,
                };
            }
            Err(other) => {
                return FcmDeliveryOutcome::PermanentReject {
                    status: 500,
                    body: other.to_string(),
                };
            }
        };

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.auth.project_id()
        );
        let payload = build_fcm_v1_message(device_token, event);
        let resp = match self
            .client
            .post(url)
            .bearer_auth(bearer)
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return FcmDeliveryOutcome::Retryable {
                    retry_after_secs: None,
                    reason: format!("Jaringan FCM error: {e}"),
                };
            }
        };

        let status = resp.status();
        let retry_after_secs = resp
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok());

        if status.is_success() {
            return FcmDeliveryOutcome::Sent;
        }
        let body = resp.text().await.unwrap_or_default();
        if status.as_u16() == 404
            || body.contains("UNREGISTERED")
            || (status.as_u16() == 400
                && body.contains("registration token is not a valid FCM registration token"))
        {
            return FcmDeliveryOutcome::PruneStaleToken;
        }
        if status.as_u16() == 401 {
            self.auth.invalidate_cache().await;
            return FcmDeliveryOutcome::Retryable {
                retry_after_secs: Some(1),
                reason: "OAuth2 token kadaluarsa, melakukan refresh".to_string(),
            };
        }
        if http_status_is_retryable(status) {
            return FcmDeliveryOutcome::Retryable {
                retry_after_secs,
                reason: format!("FCM HTTP {}: {body}", status.as_u16()),
            };
        }

        FcmDeliveryOutcome::PermanentReject {
            status: status.as_u16(),
            body,
        }
    }
}
