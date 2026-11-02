use secrecy::SecretString;
use std::env;

const DEFAULT_DATABASE_URL: &str = "postgresql://neondb_owner:npg_pv3TaxJgw0rD@ep-long-term-azhzlfs5-pooler.c-3.ap-southeast-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require";
const DEFAULT_JWT_SEED: &str = "coffee_labs_ed25519_enterprise_seed_key_2026_v2";

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: SecretString,
    pub jwt_seed: SecretString,
    pub port: u16,
    pub environment: String,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u32,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let raw_db = env::var("DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
        let raw_seed = env::var("JWT_SECRET_SEED")
            .unwrap_or_else(|_| DEFAULT_JWT_SEED.to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(3000);
        let environment = env::var("APP_ENV")
            .unwrap_or_else(|_| "production".to_string());
        let rate_limit_per_second = env::var("RATE_LIMIT_PER_SEC")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let rate_limit_burst = env::var("RATE_LIMIT_BURST")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(100);

        Self {
            database_url: SecretString::from(raw_db),
            jwt_seed: SecretString::from(raw_seed),
            port,
            environment,
            rate_limit_per_second,
            rate_limit_burst,
        }
    }
}
