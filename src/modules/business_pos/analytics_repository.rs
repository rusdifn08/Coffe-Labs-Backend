use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::modules::business_pos::analytics_dto::{StoreAnalyticsSummaryResponse, TopSellingItemDto};

pub struct BusinessAnalyticsRepository;

impl BusinessAnalyticsRepository {
    pub async fn get_summary(
        pool: &PgPool,
        store_id: Uuid,
        store_name: String,
    ) -> AppResult<StoreAnalyticsSummaryResponse> {
        // 1. Total penjualan hari ini
        let today_stats: (i64, i64) = sqlx::query_as(
            r#"SELECT 
                COALESCE(COUNT(*), 0)::int8, 
                COALESCE(SUM(total_cents), 0)::int8 
               FROM pos_orders 
               WHERE store_id = $1 AND created_at >= CURRENT_DATE"#,
        )
        .bind(store_id)
        .fetch_one(pool)
        .await?;

        let today_orders = today_stats.0;
        let today_sales = today_stats.1;
        let aov = if today_orders > 0 { today_sales / today_orders } else { 0 };

        // 2. Total penjualan bulan ini
        let month_sales: (i64,) = sqlx::query_as(
            r#"SELECT COALESCE(SUM(total_cents), 0)::int8 
               FROM pos_orders 
               WHERE store_id = $1 AND created_at >= DATE_TRUNC('month', CURRENT_DATE)"#,
        )
        .bind(store_id)
        .fetch_one(pool)
        .await?;

        // 3. Jumlah item inventori yang menipis (low stock)
        let low_stock_count: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::int8 
               FROM business_inventory_items 
               WHERE store_id = $1 AND stock_quantity <= min_reorder_level"#,
        )
        .bind(store_id)
        .fetch_one(pool)
        .await?;

        // 4. Jumlah meja yang sedang terisi (occupied)
        let occupied_tables: (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*)::int8 
               FROM business_dining_tables 
               WHERE store_id = $1 AND status = 'occupied'"#,
        )
        .bind(store_id)
        .fetch_one(pool)
        .await?;

        // 5. Top 5 Menu terlaris
        let top_rows: Vec<(String, i64, i64)> = sqlx::query_as(
            r#"SELECT 
                poi.item_name,
                SUM(poi.quantity)::int8 AS total_qty,
                SUM(poi.subtotal_cents)::int8 AS total_rev
               FROM pos_order_items poi
               JOIN pos_orders po ON po.id = poi.order_id
               WHERE po.store_id = $1
               GROUP BY poi.item_name
               ORDER BY total_qty DESC
               LIMIT 5"#,
        )
        .bind(store_id)
        .fetch_all(pool)
        .await?;

        let top_selling = top_rows
            .into_iter()
            .map(|r| TopSellingItemDto {
                item_name: r.0,
                total_quantity_sold: r.1,
                total_revenue_cents: r.2,
            })
            .collect();

        Ok(StoreAnalyticsSummaryResponse {
            store_id,
            store_name,
            today_sales_cents: today_sales,
            today_orders_count: today_orders,
            avg_order_value_cents: aov,
            month_sales_cents: month_sales.0,
            low_stock_items_count: low_stock_count.0,
            occupied_tables_count: occupied_tables.0,
            top_selling_items: top_selling,
        })
    }
}
