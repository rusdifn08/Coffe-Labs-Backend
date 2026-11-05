use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::business_pos::models::BusinessStoreConfig;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TenantContext {
    pub store_id: Uuid,
    pub owner_user_id: Uuid,
    pub store_name: String,
}

impl TenantContext {
    /// Menyelesaikan konteks tenant secara aman dari ID Pengguna JWT.
    /// Memastikan semua kueri berikutnya terisolasi 100% per kedai bisnis.
    pub async fn resolve(pool: &PgPool, user_id: Uuid) -> AppResult<Self> {
        let existing = sqlx::query_as::<_, BusinessStoreConfig>(
            "SELECT * FROM business_store_configs WHERE owner_user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        if let Some(config) = existing {
            return Ok(Self {
                store_id: config.id,
                owner_user_id: config.owner_user_id,
                store_name: config.store_name,
            });
        }

        // Auto-provision profil tenant default jika akun bisnis baru pertama kali masuk
        let default_hash = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$6x+r14227nKzJkU/s01r5lI3W2i+aRz/J+4q57mJj+E";
        let new_config = sqlx::query_as::<_, BusinessStoreConfig>(
            r#"INSERT INTO business_store_configs 
                (owner_user_id, store_name, cashier_pin_hash, aod_grid_columns, aod_grid_rows)
               VALUES ($1, 'Coffee Labs Bar', $2, 3, 2)
               RETURNING *"#,
        )
        .bind(user_id)
        .bind(default_hash)
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Gagal auto-provision tenant kedai: {}", e)))?;

        Ok(Self {
            store_id: new_config.id,
            owner_user_id: new_config.owner_user_id,
            store_name: new_config.store_name,
        })
    }
}
