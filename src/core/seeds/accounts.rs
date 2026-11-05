use secrecy::SecretString;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

use crate::core::{
    error::AppResult,
    security::{cache_verified_password, hash_password, needs_fast_rehash},
};

pub async fn seed_admin(pool: &PgPool) -> AppResult<()> {
    let admin_username = "rusdifn08";
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(admin_username)
        .fetch_one(pool)
        .await?;

    let pwd = SecretString::from("121234aa".to_string());
    if !exists {
        let password_hash = hash_password(&pwd)?;
        cache_verified_password(&pwd, &password_hash);
        sqlx::query(
            r#"INSERT INTO users (username, password_hash, full_name, role, status, tier, avatar_url)
               VALUES ($1, $2, 'Admin Rusdi', 'admin', 'approved', 'business', 'assets/images/characters/avatar_1.png')"#,
        )
        .bind(admin_username)
        .bind(&password_hash)
        .execute(pool)
        .await?;
        info!("🔐 Akun Admin default dibuat: {}", admin_username);
    } else {
        let existing_hash: Option<String> = sqlx::query_scalar("SELECT password_hash FROM users WHERE username = $1")
            .bind(admin_username)
            .fetch_optional(pool)
            .await?;
        if let Some(h) = existing_hash {
            if needs_fast_rehash(&h) {
                if let Ok(fast_hash) = hash_password(&pwd) {
                    cache_verified_password(&pwd, &fast_hash);
                    let _ = sqlx::query("UPDATE users SET password_hash = $1, role = 'admin', status = 'approved', tier = 'business' WHERE username = $2")
                        .bind(&fast_hash)
                        .bind(admin_username)
                        .execute(pool)
                        .await;
                    return Ok(());
                }
            } else {
                cache_verified_password(&pwd, &h);
            }
        }
        sqlx::query("UPDATE users SET role = 'admin', status = 'approved', tier = 'business' WHERE username = $1")
            .bind(admin_username)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn seed_business_demo_account(pool: &PgPool) -> AppResult<()> {
    let username = "bisnis123";
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
        .bind(username)
        .fetch_one(pool)
        .await?;

    let pwd = SecretString::from("bisnis123".to_string());
    if !exists {
        let password_hash = hash_password(&pwd)?;
        cache_verified_password(&pwd, &password_hash);
        let user_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO users (username, password_hash, full_name, role, status, tier, subscription_plan_id, avatar_url)
               VALUES ($1, $2, 'Coffee Labs Business Demo', 'user', 'approved', 'business', 'plan_business', 'assets/images/characters/avatar_2.png')
               RETURNING id"#,
        )
        .bind(username)
        .bind(&password_hash)
        .fetch_one(pool)
        .await?;

        let _ = sqlx::query(
            r#"INSERT INTO business_store_configs 
                (owner_user_id, store_name, cashier_pin_hash, aod_grid_columns, aod_grid_rows)
               VALUES ($1, 'Coffee Labs Bar & Kitchen', '$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$6x+r14227nKzJkU/s01r5lI3W2i+aRz/J+4q57mJj+E', 3, 2)
               ON CONFLICT (owner_user_id) DO NOTHING"#,
        )
        .bind(user_id)
        .execute(pool)
        .await;

        info!("🏢 Akun Demo Business berhasil dibuat: {} (Password: {})", username, "bisnis123");
    } else {
        let existing_hash: Option<String> = sqlx::query_scalar("SELECT password_hash FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(pool)
            .await?;
        if let Some(h) = existing_hash {
            if needs_fast_rehash(&h) {
                if let Ok(fast_hash) = hash_password(&pwd) {
                    cache_verified_password(&pwd, &fast_hash);
                    let _ = sqlx::query("UPDATE users SET password_hash = $1, status = 'approved', tier = 'business', subscription_plan_id = 'plan_business' WHERE username = $2")
                        .bind(&fast_hash)
                        .bind(username)
                        .execute(pool)
                        .await;
                }
            } else {
                cache_verified_password(&pwd, &h);
            }
        }
        sqlx::query("UPDATE users SET status = 'approved', tier = 'business', subscription_plan_id = 'plan_business' WHERE username = $1")
            .bind(username)
            .execute(pool)
            .await?;
    }
    Ok(())
}
