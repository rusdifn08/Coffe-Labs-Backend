use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::modules::business_pos::inventory_models::BusinessInventoryItem;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInventoryItemDto {
    #[validate(length(min = 2, max = 100, message = "Nama item minimal 2 karakter, maksimal 100."))]
    pub name: String,
    pub sku: Option<String>,
    #[validate(length(min = 2, max = 50, message = "Kategori minimal 2 karakter."))]
    pub category: String,
    #[validate(range(min = 0.0, message = "Kuantitas stok tidak boleh negatif."))]
    pub stock_quantity: f64,
    #[validate(length(min = 1, max = 20, message = "Satuan harus diisi (e.g. gram, liter, pcs)."))]
    pub unit: String,
    #[validate(range(min = 0.0, message = "Ambang batas reorder tidak boleh negatif."))]
    pub min_reorder_level: f64,
    #[validate(range(min = 0, message = "Biaya satuan tidak boleh negatif."))]
    pub cost_per_unit_cents: i64,
    pub supplier_name: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[allow(dead_code)]
pub struct AdjustStockDto {
    pub delta_quantity: f64,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InventoryItemResponse {
    pub id: Uuid,
    pub store_id: Uuid,
    pub name: String,
    pub sku: Option<String>,
    pub category: String,
    pub stock_quantity: f64,
    pub unit: String,
    pub min_reorder_level: f64,
    pub is_low_stock: bool,
    pub cost_per_unit_cents: i64,
    pub supplier_name: Option<String>,
    pub last_restocked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BusinessInventoryItem> for InventoryItemResponse {
    fn from(item: BusinessInventoryItem) -> Self {
        let is_low_stock = item.stock_quantity <= item.min_reorder_level;
        Self {
            id: item.id,
            store_id: item.store_id,
            name: item.name,
            sku: item.sku,
            category: item.category,
            stock_quantity: item.stock_quantity,
            unit: item.unit,
            min_reorder_level: item.min_reorder_level,
            is_low_stock,
            cost_per_unit_cents: item.cost_per_unit_cents,
            supplier_name: item.supplier_name,
            last_restocked_at: item.last_restocked_at,
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}
