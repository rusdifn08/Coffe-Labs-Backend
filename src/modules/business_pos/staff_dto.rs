use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::modules::business_pos::staff_models::BusinessStaffMember;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateStaffDto {
    #[validate(length(min = 2, max = 100, message = "Nama barista/staf minimal 2 karakter."))]
    pub full_name: String,
    #[validate(length(min = 2, max = 50, message = "Jabatan (role) minimal 2 karakter."))]
    pub role_title: String,
    #[validate(length(min = 4, max = 8, message = "PIN kasir staf harus 4-8 digit angka."))]
    pub passcode_pin: String,
    pub can_give_discounts: Option<bool>,
    pub can_void_orders: Option<bool>,
    pub can_view_reports: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateStaffDto {
    pub full_name: Option<String>,
    pub role_title: Option<String>,
    pub passcode_pin: Option<String>,
    pub can_give_discounts: Option<bool>,
    pub can_void_orders: Option<bool>,
    pub can_view_reports: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct StaffResponse {
    pub id: Uuid,
    pub store_id: Uuid,
    pub full_name: String,
    pub role_title: String,
    pub can_give_discounts: bool,
    pub can_void_orders: bool,
    pub can_view_reports: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<BusinessStaffMember> for StaffResponse {
    fn from(s: BusinessStaffMember) -> Self {
        Self {
            id: s.id,
            store_id: s.store_id,
            full_name: s.full_name,
            role_title: s.role_title,
            can_give_discounts: s.can_give_discounts,
            can_void_orders: s.can_void_orders,
            can_view_reports: s.can_view_reports,
            is_active: s.is_active,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}
