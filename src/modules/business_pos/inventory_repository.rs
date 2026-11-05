use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::business_pos::{
    inventory_dto::CreateInventoryItemDto, inventory_models::BusinessInventoryItem,
};

pub struct BusinessInventoryRepository;

impl BusinessInventoryRepository {
    pub async fn list_by_store(
        pool: &PgPool,
        store_id: Uuid,
        category: Option<&str>,
    ) -> AppResult<Vec<BusinessInventoryItem>> {
        let items = if let Some(cat) = category {
            sqlx::query_as::<_, BusinessInventoryItem>(
                r#"SELECT * FROM business_inventory_items 
                   WHERE store_id = $1 AND category = $2 
                   ORDER BY name ASC"#,
            )
            .bind(store_id)
            .bind(cat)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, BusinessInventoryItem>(
                r#"SELECT * FROM business_inventory_items 
                   WHERE store_id = $1 
                   ORDER BY category ASC, name ASC"#,
            )
            .bind(store_id)
            .fetch_all(pool)
            .await?
        };

        Ok(items)
    }

    pub async fn create(
        pool: &PgPool,
        store_id: Uuid,
        dto: &CreateInventoryItemDto,
    ) -> AppResult<BusinessInventoryItem> {
        let item = sqlx::query_as::<_, BusinessInventoryItem>(
            r#"INSERT INTO business_inventory_items 
                (store_id, name, sku, category, stock_quantity, unit, min_reorder_level, cost_per_unit_cents, supplier_name, last_restocked_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
               RETURNING *"#,
        )
        .bind(store_id)
        .bind(&dto.name)
        .bind(&dto.sku)
        .bind(&dto.category)
        .bind(dto.stock_quantity)
        .bind(&dto.unit)
        .bind(dto.min_reorder_level)
        .bind(dto.cost_per_unit_cents)
        .bind(&dto.supplier_name)
        .fetch_one(pool)
        .await?;

        Ok(item)
    }

    pub async fn adjust_stock(
        pool: &PgPool,
        store_id: Uuid,
        item_id: Uuid,
        delta: f64,
    ) -> AppResult<BusinessInventoryItem> {
        let item = sqlx::query_as::<_, BusinessInventoryItem>(
            r#"UPDATE business_inventory_items 
               SET stock_quantity = GREATEST(0.0, stock_quantity + $1),
                   last_restocked_at = CASE WHEN $1 > 0 THEN NOW() ELSE last_restocked_at END,
                   updated_at = NOW()
               WHERE store_id = $2 AND id = $3
               RETURNING *"#,
        )
        .bind(delta)
        .bind(store_id)
        .bind(item_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Item inventori tidak ditemukan pada store Anda.".to_string()))?;

        Ok(item)
    }

    pub async fn delete(pool: &PgPool, store_id: Uuid, item_id: Uuid) -> AppResult<bool> {
        let res = sqlx::query(
            "DELETE FROM business_inventory_items WHERE store_id = $1 AND id = $2",
        )
        .bind(store_id)
        .bind(item_id)
        .execute(pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }
}
