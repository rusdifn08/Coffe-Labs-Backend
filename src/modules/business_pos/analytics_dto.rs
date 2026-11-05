use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct TopSellingItemDto {
    pub item_name: String,
    pub total_quantity_sold: i64,
    pub total_revenue_cents: i64,
}

#[derive(Debug, Serialize)]
pub struct StoreAnalyticsSummaryResponse {
    pub store_id: Uuid,
    pub store_name: String,
    pub today_sales_cents: i64,
    pub today_orders_count: i64,
    pub avg_order_value_cents: i64,
    pub month_sales_cents: i64,
    pub low_stock_items_count: i64,
    pub occupied_tables_count: i64,
    pub top_selling_items: Vec<TopSellingItemDto>,
}
