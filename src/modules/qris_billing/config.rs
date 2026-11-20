use std::env;

use sha2::{Digest, Sha256};

use crate::core::config::AppConfig;
use crate::modules::qris_billing::emvco_qris::DEFAULT_COFFEELABS_STATIC_QRIS;

#[derive(Debug, Clone)]
pub struct LivinImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub mailbox: String,
}

#[derive(Debug, Clone)]
pub struct QrisBillingConfig {
    pub master_static_qris_payload: String,
    pub bridge_hmac_secret: String,
    pub imap: Option<LivinImapConfig>,
}

fn env_non_empty(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl QrisBillingConfig {
    pub fn from_app_config(app_cfg: &AppConfig) -> Self {
        let master_static_qris_payload = env_non_empty("MASTER_STATIC_QRIS_PAYLOAD")
            .unwrap_or_else(|| DEFAULT_COFFEELABS_STATIC_QRIS.to_string());

        let bridge_hmac_secret = env_non_empty("QRIS_BRIDGE_HMAC_SECRET").unwrap_or_else(|| {
            let mut hasher = Sha256::new();
            hasher.update(b"coffeelabs:qris_bridge_v1:");
            hasher.update(app_cfg.jwt_secret_str().as_bytes());
            hex::encode(hasher.finalize())
        });

        let imap = match (
            env_non_empty("LIVIN_IMAP_HOST"),
            env_non_empty("LIVIN_IMAP_USERNAME"),
            env_non_empty("LIVIN_IMAP_PASSWORD"),
        ) {
            (Some(host), Some(username), Some(password)) => {
                let port = env_non_empty("LIVIN_IMAP_PORT")
                    .and_then(|p| p.parse::<u16>().ok())
                    .unwrap_or(993);
                let mailbox = env_non_empty("LIVIN_IMAP_MAILBOX")
                    .unwrap_or_else(|| "INBOX".to_string());
                Some(LivinImapConfig {
                    host,
                    port,
                    username,
                    password,
                    mailbox,
                })
            }
            _ => None,
        };

        Self {
            master_static_qris_payload,
            bridge_hmac_secret,
            imap,
        }
    }
}
