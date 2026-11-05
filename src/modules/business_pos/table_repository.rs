use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::business_pos::{
    table_dto::{CreateTableDto, UpdateTableStatusDto},
    table_models::BusinessDiningTable,
};

pub struct BusinessTableRepository;

impl BusinessTableRepository {
    pub async fn list_by_store(
        pool: &PgPool,
        store_id: Uuid,
    ) -> AppResult<Vec<BusinessDiningTable>> {
        let tables = sqlx::query_as::<_, BusinessDiningTable>(
            r#"SELECT * FROM business_dining_tables 
               WHERE store_id = $1 
               ORDER BY section ASC, table_number ASC"#,
        )
        .bind(store_id)
        .fetch_all(pool)
        .await?;

        Ok(tables)
    }

    pub async fn create(
        pool: &PgPool,
        store_id: Uuid,
        dto: &CreateTableDto,
    ) -> AppResult<BusinessDiningTable> {
        let qr_token = format!("{}_{}", store_id, dto.table_number.to_lowercase().replace(' ', "_"));
        let table = sqlx::query_as::<_, BusinessDiningTable>(
            r#"INSERT INTO business_dining_tables 
                (store_id, table_number, section, capacity, status, qr_code_token)
               VALUES ($1, $2, $3, $4, 'available', $5)
               ON CONFLICT (store_id, table_number) DO UPDATE
               SET section = EXCLUDED.section, capacity = EXCLUDED.capacity, updated_at = NOW()
               RETURNING *"#,
        )
        .bind(store_id)
        .bind(&dto.table_number)
        .bind(&dto.section)
        .bind(dto.capacity)
        .bind(&qr_token)
        .fetch_one(pool)
        .await?;

        Ok(table)
    }

    pub async fn update_status(
        pool: &PgPool,
        store_id: Uuid,
        table_id: Uuid,
        dto: &UpdateTableStatusDto,
    ) -> AppResult<BusinessDiningTable> {
        let table = sqlx::query_as::<_, BusinessDiningTable>(
            r#"UPDATE business_dining_tables 
               SET status = $1,
                   active_order_id = $2,
                   updated_at = NOW()
               WHERE store_id = $3 AND id = $4
               RETURNING *"#,
        )
        .bind(&dto.status)
        .bind(dto.active_order_id)
        .bind(store_id)
        .bind(table_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Meja tidak ditemukan pada store Anda.".to_string()))?;

        Ok(table)
    }

    pub async fn delete(pool: &PgPool, store_id: Uuid, table_id: Uuid) -> AppResult<bool> {
        let res = sqlx::query(
            "DELETE FROM business_dining_tables WHERE store_id = $1 AND id = $2",
        )
        .bind(store_id)
        .bind(table_id)
        .execute(pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }
}
