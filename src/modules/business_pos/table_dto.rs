use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTableDto {
    #[validate(length(min = 1, max = 20, message = "Nomor meja harus 1 - 20 karakter."))]
    pub table_number: String,
    #[validate(length(min = 2, max = 50, message = "Area/seksi minimal 2 karakter (contoh: Indoor Bar, Outdoor Patio)."))]
    pub section: String,
    #[validate(range(min = 1, max = 50, message = "Kapasitas meja antara 1 dan 50 orang."))]
    pub capacity: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTableStatusDto {
    #[validate(length(min = 2, max = 20, message = "Status harus 'available', 'occupied', 'reserved', atau 'cleaning'."))]
    pub status: String,
    pub active_order_id: Option<Uuid>,
}
