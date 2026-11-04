#![allow(dead_code)]
use sqlx::PgPool;
use uuid::Uuid;
use crate::core::error::AppResult;
use crate::modules::business_pos::models::{BusinessMenuItem, BusinessStoreConfig, PosOrder};

pub struct BusinessPosRepository;

impl BusinessPosRepository {
    pub async fn get_or_create_config(pool: &PgPool, owner_id: Uuid) -> AppResult<BusinessStoreConfig> {
        let existing = sqlx::query_as::<_, BusinessStoreConfig>(
            "SELECT * FROM business_store_configs WHERE owner_user_id = $1"
        )
        .bind(owner_id)
        .fetch_optional(pool)
        .await?;

        if let Some(config) = existing {
            return Ok(config);
        }

        // Default PIN: 1234 (Argon2id hash)
        let default_hash = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$6x+r14227nKzJkU/s01r5lI3W2i+aRz/J+4q57mJj+E";
        let new_config = sqlx::query_as::<_, BusinessStoreConfig>(
            r#"INSERT INTO business_store_configs 
                (owner_user_id, store_name, cashier_pin_hash, aod_grid_columns, aod_grid_rows)
               VALUES ($1, 'Coffee Labs Bar', $2, 3, 2)
               RETURNING *"#
        )
        .bind(owner_id)
        .bind(default_hash)
        .fetch_one(pool)
        .await?;

        Ok(new_config)
    }

    pub async fn update_config(
        pool: &PgPool,
        owner_id: Uuid,
        name: &str,
        cols: i32,
        rows: i32,
        border: &str,
        accent: &str,
        idle_br: f32,
        active_br: f32,
        pixel_shift: bool,
        pin_hash: Option<&str>,
    ) -> AppResult<BusinessStoreConfig> {
        let updated = if let Some(hash) = pin_hash {
            sqlx::query_as::<_, BusinessStoreConfig>(
                r#"UPDATE business_store_configs
                   SET store_name = $2, aod_grid_columns = $3, aod_grid_rows = $4,
                       aod_card_border_color = $5, aod_accent_color = $6,
                       aod_idle_brightness = $7, aod_active_brightness = $8,
                       pixel_shift_enabled = $9, cashier_pin_hash = $10, updated_at = NOW()
                   WHERE owner_user_id = $1
                   RETURNING *"#
            )
            .bind(owner_id).bind(name).bind(cols).bind(rows).bind(border).bind(accent)
            .bind(idle_br).bind(active_br).bind(pixel_shift).bind(hash)
            .fetch_one(pool).await?
        } else {
            sqlx::query_as::<_, BusinessStoreConfig>(
                r#"UPDATE business_store_configs
                   SET store_name = $2, aod_grid_columns = $3, aod_grid_rows = $4,
                       aod_card_border_color = $5, aod_accent_color = $6,
                       aod_idle_brightness = $7, aod_active_brightness = $8,
                       pixel_shift_enabled = $9, updated_at = NOW()
                   WHERE owner_user_id = $1
                   RETURNING *"#
            )
            .bind(owner_id).bind(name).bind(cols).bind(rows).bind(border).bind(accent)
            .bind(idle_br).bind(active_br).bind(pixel_shift)
            .fetch_one(pool).await?
        };

        Ok(updated)
    }

    pub async fn list_menu_items(pool: &PgPool, store_id: Uuid) -> AppResult<Vec<BusinessMenuItem>> {
        let items = sqlx::query_as::<_, BusinessMenuItem>(
            "SELECT * FROM business_menu_items WHERE store_id = $1 ORDER BY display_order ASC, created_at ASC"
        )
        .bind(store_id)
        .fetch_all(pool)
        .await?;

        Ok(items)
    }

    pub async fn create_menu_item(
        pool: &PgPool,
        store_id: Uuid,
        name: &str,
        desc: &str,
        cat: &str,
        price: i64,
        curr: &str,
        asset_url: &str,
        format: &str,
        order: i32,
    ) -> AppResult<BusinessMenuItem> {
        let item = sqlx::query_as::<_, BusinessMenuItem>(
            r#"INSERT INTO business_menu_items 
                (store_id, name, description, category, price_cents, currency, asset_url, asset_format, display_order)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING *"#
        )
        .bind(store_id).bind(name).bind(desc).bind(cat).bind(price)
        .bind(curr).bind(asset_url).bind(format).bind(order)
        .fetch_one(pool)
        .await?;

        Ok(item)
    }

    pub async fn delete_menu_item(pool: &PgPool, store_id: Uuid, item_id: Uuid) -> AppResult<bool> {
        let res = sqlx::query(
            "DELETE FROM business_menu_items WHERE id = $1 AND store_id = $2"
        )
        .bind(item_id)
        .bind(store_id)
        .execute(pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn create_pos_order(
        pool: &PgPool,
        store_id: Uuid,
        order_num: &str,
        cashier_id: Option<Uuid>,
        cust_name: Option<&str>,
        subtotal: i64,
        total: i64,
        pay_method: &str,
        notes: Option<&str>,
    ) -> AppResult<PosOrder> {
        let order = sqlx::query_as::<_, PosOrder>(
            r#"INSERT INTO pos_orders 
                (store_id, order_number, cashier_user_id, customer_name, subtotal_cents, total_cents, payment_method, notes)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING *"#
        )
        .bind(store_id).bind(order_num).bind(cashier_id).bind(cust_name)
        .bind(subtotal).bind(total).bind(pay_method).bind(notes)
        .fetch_one(pool)
        .await?;

        Ok(order)
    }

    pub async fn get_pin_hash(pool: &PgPool, store_id: Uuid) -> AppResult<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT cashier_pin_hash FROM business_store_configs WHERE id = $1"
        )
        .bind(store_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.0))
    }
}
