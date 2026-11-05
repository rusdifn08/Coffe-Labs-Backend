use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::business_pos::{
    staff_dto::{CreateStaffDto, UpdateStaffDto},
    staff_models::BusinessStaffMember,
};

pub struct BusinessStaffRepository;

impl BusinessStaffRepository {
    pub async fn list_by_store(
        pool: &PgPool,
        store_id: Uuid,
    ) -> AppResult<Vec<BusinessStaffMember>> {
        let staff = sqlx::query_as::<_, BusinessStaffMember>(
            r#"SELECT * FROM business_staff_members 
               WHERE store_id = $1 
               ORDER BY is_active DESC, full_name ASC"#,
        )
        .bind(store_id)
        .fetch_all(pool)
        .await?;

        Ok(staff)
    }

    pub async fn create(
        pool: &PgPool,
        store_id: Uuid,
        dto: &CreateStaffDto,
        pin_hash: &str,
    ) -> AppResult<BusinessStaffMember> {
        let member = sqlx::query_as::<_, BusinessStaffMember>(
            r#"INSERT INTO business_staff_members 
                (store_id, full_name, role_title, passcode_pin_hash, can_give_discounts, can_void_orders, can_view_reports, is_active)
               VALUES ($1, $2, $3, $4, $5, $6, $7, true)
               RETURNING *"#,
        )
        .bind(store_id)
        .bind(&dto.full_name)
        .bind(&dto.role_title)
        .bind(pin_hash)
        .bind(dto.can_give_discounts.unwrap_or(false))
        .bind(dto.can_void_orders.unwrap_or(false))
        .bind(dto.can_view_reports.unwrap_or(false))
        .fetch_one(pool)
        .await?;

        Ok(member)
    }

    pub async fn update(
        pool: &PgPool,
        store_id: Uuid,
        staff_id: Uuid,
        dto: &UpdateStaffDto,
        pin_hash: Option<&str>,
    ) -> AppResult<BusinessStaffMember> {
        let existing = sqlx::query_as::<_, BusinessStaffMember>(
            "SELECT * FROM business_staff_members WHERE store_id = $1 AND id = $2",
        )
        .bind(store_id)
        .bind(staff_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Data staf barista tidak ditemukan.".to_string()))?;

        let full_name = dto.full_name.as_deref().unwrap_or(&existing.full_name);
        let role_title = dto.role_title.as_deref().unwrap_or(&existing.role_title);
        let hash = pin_hash.unwrap_or(&existing.passcode_pin_hash);
        let can_disc = dto.can_give_discounts.unwrap_or(existing.can_give_discounts);
        let can_void = dto.can_void_orders.unwrap_or(existing.can_void_orders);
        let can_view = dto.can_view_reports.unwrap_or(existing.can_view_reports);
        let active = dto.is_active.unwrap_or(existing.is_active);

        let updated = sqlx::query_as::<_, BusinessStaffMember>(
            r#"UPDATE business_staff_members 
               SET full_name = $1,
                   role_title = $2,
                   passcode_pin_hash = $3,
                   can_give_discounts = $4,
                   can_void_orders = $5,
                   can_view_reports = $6,
                   is_active = $7,
                   updated_at = NOW()
               WHERE store_id = $8 AND id = $9
               RETURNING *"#,
        )
        .bind(full_name)
        .bind(role_title)
        .bind(hash)
        .bind(can_disc)
        .bind(can_void)
        .bind(can_view)
        .bind(active)
        .bind(store_id)
        .bind(staff_id)
        .fetch_one(pool)
        .await?;

        Ok(updated)
    }

    pub async fn delete(pool: &PgPool, store_id: Uuid, staff_id: Uuid) -> AppResult<bool> {
        let res = sqlx::query(
            "DELETE FROM business_staff_members WHERE store_id = $1 AND id = $2",
        )
        .bind(store_id)
        .bind(staff_id)
        .execute(pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }
}
