use secrecy::SecretString;
use std::{env, fmt, path::Path};

const DEFAULT_JWT_SEED: &str = "coffee_labs_ed25519_enterprise_seed_key_2026_v2";
const DEFAULT_FCM_PROJECT_ID: &str = "coffe-labs";

#[derive(Clone)]
pub struct CloudinaryConfig {
    pub cloud_name: String,
    pub api_key: String,
    pub api_secret: SecretString,
}

impl fmt::Debug for CloudinaryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudinaryConfig")
            .field("cloud_name", &self.cloud_name)
            .field("api_key", &"[REDACTED]")
            .field("api_secret", &"[REDACTED]")
            .finish()
    }
}

/// Mem-parse string `cloudinary://api_key:api_secret@cloud_name` secara aman tanpa panic.
pub fn parse_cloudinary_url(raw: &str) -> Result<CloudinaryConfig, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("CLOUDINARY_URL kosong.".to_string());
    }
    let rest = trimmed
        .strip_prefix("cloudinary://")
        .ok_or_else(|| "Format CLOUDINARY_URL harus diawali 'cloudinary://'".to_string())?;
    let (creds, cloud_part) = rest
        .rsplit_once('@')
        .ok_or_else(|| "Format CLOUDINARY_URL harus memuat '@cloud_name'".to_string())?;
    let (api_key_raw, api_secret_raw) = creds
        .split_once(':')
        .ok_or_else(|| "Format CLOUDINARY_URL harus memuat 'api_key:api_secret'".to_string())?;

    let api_key = api_key_raw.trim();
    let api_secret = api_secret_raw.trim();
    let cloud_name = cloud_part
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim();

    if api_key.is_empty() || api_secret.is_empty() || cloud_name.is_empty() {
        return Err("api_key, api_secret, dan cloud_name pada CLOUDINARY_URL tidak boleh kosong.".to_string());
    }
    if api_key.chars().any(char::is_whitespace) || api_secret.chars().any(char::is_whitespace) {
        return Err("Kredensial CLOUDINARY_URL tidak boleh mengandung spasi.".to_string());
    }
    if !cloud_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("cloud_name pada CLOUDINARY_URL mengandung karakter tidak valid.".to_string());
    }

    Ok(CloudinaryConfig {
        cloud_name: cloud_name.to_string(),
        api_key: api_key.to_string(),
        api_secret: SecretString::from(api_secret.to_string()),
    })
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub rabbitmq_url: Option<String>,
    pub google_application_credentials: Option<String>,
    pub fcm_project_id: String,
    pub cloudinary_url: Option<String>,
    pub jwt_seed: SecretString,
    pub port: u16,
    pub environment: String,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u32,
}

impl fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"[REDACTED]")
            .field("redis_url", &if self.redis_url.is_empty() { "[UNSET]" } else { "[REDACTED]" })
            .field("rabbitmq_url", &self.rabbitmq_url.as_ref().map(|_| "[REDACTED]"))
            .field("google_application_credentials", &self.google_application_credentials)
            .field("fcm_project_id", &self.fcm_project_id)
            .field("cloudinary_url", &self.cloudinary_url.as_ref().map(|_| "[REDACTED]"))
            .field("port", &self.port)
            .field("environment", &self.environment)
            .finish()
    }
}

fn read_optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .expect("DATABASE_URL must be set");
        let redis_url = read_optional_env("REDIS_URL").unwrap_or_default();
        let rabbitmq_url = read_optional_env("RABBITMQ_URL");
        let google_application_credentials = read_optional_env("GOOGLE_APPLICATION_CREDENTIALS")
            .or_else(|| {
                Path::new("./firebase-service-account.json")
                    .exists()
                    .then(|| "./firebase-service-account.json".to_string())
            });
        let fcm_project_id =
            read_optional_env("FCM_PROJECT_ID").unwrap_or_else(|| DEFAULT_FCM_PROJECT_ID.to_string());
        let cloudinary_url = read_optional_env("CLOUDINARY_URL");
        let raw_seed =
            env::var("JWT_SECRET_SEED").unwrap_or_else(|_| DEFAULT_JWT_SEED.to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(3000);
        let environment = env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
        let rate_limit_per_second = env::var("RATE_LIMIT_PER_SEC")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(500);
        let rate_limit_burst = env::var("RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(5000);

        Self {
            database_url,
            redis_url,
            rabbitmq_url,
            google_application_credentials,
            fcm_project_id,
            cloudinary_url,
            jwt_seed: SecretString::from(raw_seed),
            port,
            environment,
            rate_limit_per_second,
            rate_limit_burst,
        }
    }

    pub fn cloudinary_config(&self) -> Result<Option<CloudinaryConfig>, String> {
        self.cloudinary_url
            .as_deref()
            .map(parse_cloudinary_url)
            .transpose()
    }
}
