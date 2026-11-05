use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::modules::business_pos::{
    shift_dto::{CloseShiftDto, OpenShiftDto},
    shift_models::BusinessCashierShift,
};

pub struct BusinessShiftRepository;

impl BusinessShiftRepository {
    pub async fn get_active_shift(
        pool: &PgPool,
        store_id: Uuid,
    ) -> AppResult<Option<BusinessCashierShift>> {
        let shift = sqlx::query_as::<_, BusinessCashierShift>(
            r#"SELECT * FROM business_cashier_shifts 
               WHERE store_id = $1 AND status = 'open' 
               ORDER BY opened_at DESC LIMIT 1"#,
        )
        .bind(store_id)
        .fetch_optional(pool)
        .await?;

        Ok(shift)
    }

    pub async fn open_shift(
        pool: &PgPool,
        store_id: Uuid,
        cashier_id: Uuid,
        cashier_name: &str,
        dto: &OpenShiftDto,
    ) -> AppResult<BusinessCashierShift> {
        // Cek apakah sudah ada shift yang sedang aktif
        if let Some(active) = Self::get_active_shift(pool, store_id).await? {
            return Err(AppError::BadRequest(format!(
                "Shift '{}' masih aktif. Tutup shift sebelumnya sebelum membuka shift baru.",
                active.shift_name
            )));
        }

        let shift = sqlx::query_as::<_, BusinessCashierShift>(
            r#"INSERT INTO business_cashier_shifts 
                (store_id, cashier_user_id, cashier_name, shift_name, opening_cash_cents, expected_cash_cents, status, notes)
               VALUES ($1, $2, $3, $4, $5, $5, 'open', $6)
               RETURNING *"#,
        )
        .bind(store_id)
        .bind(cashier_id)
        .bind(cashier_name)
        .bind(&dto.shift_name)
        .bind(dto.opening_cash_cents)
        .bind(&dto.notes)
        .fetch_one(pool)
        .await?;

        Ok(shift)
    }

    pub async fn close_shift(
        pool: &PgPool,
        store_id: Uuid,
        shift_id: Uuid,
        dto: &CloseShiftDto,
    ) -> AppResult<BusinessCashierShift> {
        // Ambil shift aktif dan hitung total penjualan kasir selama rentang waktu shift
        let current = sqlx::query_as::<_, BusinessCashierShift>(
            "SELECT * FROM business_cashier_shifts WHERE store_id = $1 AND id = $2 AND status = 'open'",
        )
        .bind(store_id)
        .bind(shift_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Shift kasir aktif tidak ditemukan pada store Anda.".to_string()))?;

        // Hitung total penjualan tunai dari pos_orders selama shift berjalan
        let sales_stats: (i64, i64) = sqlx::query_as(
            r#"SELECT 
                COALESCE(COUNT(*), 0)::int8, 
                COALESCE(SUM(total_cents), 0)::int8 
               FROM pos_orders 
               WHERE store_id = $1 AND created_at >= $2"#,
        )
        .bind(store_id)
        .bind(current.opened_at)
        .fetch_one(pool)
        .await?;

        let total_orders = sales_stats.0 as i32;
        let total_sales = sales_stats.1;
        let expected_cash = current.opening_cash_cents + total_sales;
        let diff = dto.actual_cash_cents - expected_cash;

        let closed = sqlx::query_as::<_, BusinessCashierShift>(
            r#"UPDATE business_cashier_shifts 
               SET closed_at = NOW(),
                   actual_cash_cents = $1,
                   expected_cash_cents = $2,
                   cash_difference_cents = $3,
                   total_orders_count = $4,
                   total_sales_cents = $5,
                   status = 'closed',
                   notes = COALESCE($6, notes)
               WHERE store_id = $7 AND id = $8
               RETURNING *"#,
        )
        .bind(dto.actual_cash_cents)
        .bind(expected_cash)
        .bind(diff)
        .bind(total_orders)
        .bind(total_sales)
        .bind(&dto.notes)
        .bind(store_id)
        .bind(shift_id)
        .fetch_one(pool)
        .await?;

        Ok(closed)
    }

    pub async fn list_history(
        pool: &PgPool,
        store_id: Uuid,
        limit: i64,
    ) -> AppResult<Vec<BusinessCashierShift>> {
        let history = sqlx::query_as::<_, BusinessCashierShift>(
            r#"SELECT * FROM business_cashier_shifts 
               WHERE store_id = $1 
               ORDER BY opened_at DESC 
               LIMIT $2"#,
        )
        .bind(store_id)
        .bind(limit.clamp(1, 100))
        .fetch_all(pool)
        .await?;

        Ok(history)
    }
}
