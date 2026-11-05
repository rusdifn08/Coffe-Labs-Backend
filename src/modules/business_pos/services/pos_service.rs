use secrecy::SecretString;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::core::{
    error::{AppError, AppResult},
    security::{hash_password, verify_password},
};
use crate::modules::business_pos::{
    dto::{CreateMenuItemDto, CreatePosOrderDto, UpdateBusinessConfigDto},
    models::{BusinessMenuItem, BusinessStoreConfig, PosOrder},
    repository::BusinessPosRepository,
};

pub struct BusinessPosService;

impl BusinessPosService {
    pub async fn get_config(pool: &PgPool, owner_id: Uuid) -> AppResult<BusinessStoreConfig> {
        BusinessPosRepository::get_or_create_config(pool, owner_id).await
    }

    pub async fn update_config(
        pool: &PgPool,
        owner_id: Uuid,
        dto: UpdateBusinessConfigDto,
    ) -> AppResult<BusinessStoreConfig> {
        let current = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;

        let pin_hash = if let Some(new_pin) = dto.new_cashier_pin {
            let secret = SecretString::new(new_pin.into());
            Some(hash_password(&secret)?)
        } else {
            None
        };

        let updated = BusinessPosRepository::update_config(
            pool,
            owner_id,
            dto.store_name.as_deref().unwrap_or(&current.store_name),
            dto.aod_grid_columns.unwrap_or(current.aod_grid_columns),
            dto.aod_grid_rows.unwrap_or(current.aod_grid_rows),
            dto.aod_card_border_color.as_deref().unwrap_or(&current.aod_card_border_color),
            dto.aod_accent_color.as_deref().unwrap_or(&current.aod_accent_color),
            dto.aod_idle_brightness.unwrap_or(current.aod_idle_brightness),
            dto.aod_active_brightness.unwrap_or(current.aod_active_brightness),
            dto.pixel_shift_enabled.unwrap_or(current.pixel_shift_enabled),
            pin_hash.as_deref(),
        )
        .await?;

        Ok(updated)
    }

    pub async fn list_menu(pool: &PgPool, owner_id: Uuid) -> AppResult<Vec<BusinessMenuItem>> {
        let config = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;
        BusinessPosRepository::list_menu_items(pool, config.id).await
    }

    pub async fn add_menu_item(
        pool: &PgPool,
        owner_id: Uuid,
        dto: CreateMenuItemDto,
    ) -> AppResult<BusinessMenuItem> {
        let config = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;
        BusinessPosRepository::create_menu_item(
            pool,
            config.id,
            &dto.name,
            dto.description.as_deref().unwrap_or(""),
            &dto.category,
            dto.price_cents,
            dto.currency.as_deref().unwrap_or("IDR"),
            &dto.asset_url,
            &dto.asset_format,
            dto.display_order.unwrap_or(0),
            dto.show_in_aod.unwrap_or(true),
            dto.show_in_pos.unwrap_or(true),
        )
        .await
    }

    pub async fn update_menu_item(
        pool: &PgPool,
        owner_id: Uuid,
        item_id: Uuid,
        dto: crate::modules::business_pos::dto::UpdateMenuItemDto,
    ) -> AppResult<Option<BusinessMenuItem>> {
        let config = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;
        BusinessPosRepository::update_menu_item(
            pool,
            config.id,
            item_id,
            dto.show_in_aod,
            dto.show_in_pos,
            dto.is_available,
        )
        .await
    }

    pub async fn delete_menu_item(pool: &PgPool, owner_id: Uuid, item_id: Uuid) -> AppResult<bool> {
        let config = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;
        BusinessPosRepository::delete_menu_item(pool, config.id, item_id).await
    }

    pub async fn verify_pin(pool: &PgPool, owner_id: Uuid, raw_pin: &str) -> AppResult<bool> {
        let config = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;
        let secret = SecretString::new(raw_pin.to_string().into());
        let is_valid = verify_password(&secret, &config.cashier_pin_hash);
        if !is_valid {
            return Err(AppError::Unauthorized("PIN Kasir tidak sesuai.".to_string()));
        }
        Ok(true)
    }

    pub async fn create_pos_order(
        pool: &PgPool,
        owner_id: Uuid,
        dto: CreatePosOrderDto,
    ) -> AppResult<PosOrder> {
        let config = BusinessPosRepository::get_or_create_config(pool, owner_id).await?;
        
        let subtotal: i64 = dto.items.iter().map(|item| item.quantity as i64 * item.unit_price_cents).sum();
        let total = subtotal; // can add tax if needed
        let order_number = format!("ORD-{}", Utc::now().timestamp_millis() % 1000000);

        BusinessPosRepository::create_pos_order(
            pool,
            config.id,
            &order_number,
            Some(owner_id),
            dto.customer_name.as_deref(),
            subtotal,
            total,
            dto.payment_method.as_deref().unwrap_or("Cash"),
            dto.notes.as_deref(),
        )
        .await
    }
}
