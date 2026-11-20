use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct SubscriptionPlanRow {
    pub plan_code: String,
    pub target_tier: String,
    pub display_name: String,
    pub duration_days: i32,
    pub base_price_idr: i64,
    pub original_price_idr: Option<i64>,
    pub promo_badge_text: Option<String>,
    pub is_highlighted: bool,
    pub features_json: Value,
    pub is_active: bool,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct QrisInvoiceRow {
    pub id: Uuid,
    pub invoice_number: String,
    pub user_id: Uuid,
    pub plan_code: String,
    pub target_tier: String,
    pub duration_days: i32,
    pub base_amount_idr: i64,
    pub unique_code_idr: i16,
    pub total_payable_idr: i64,
    pub dynamic_qris_payload: String,
    pub merchant_name: String,
    pub acquirer_code: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub matched_mutation_id: Option<Uuid>,
    pub manual_proof_cloudinary_url: Option<String>,
    pub receipt_pdf_cloudinary_url: Option<String>,
    pub applied_referral_code: Option<String>,
    pub referrer_user_id: Option<Uuid>,
    pub buyer_discount_idr: i64,
    pub affiliate_commission_bps: i32,
    pub affiliate_commission_idr: i64,
    pub is_commission_credited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct QrisMutationRow {
    pub id: Uuid,
    pub source_channel: String,
    pub source_fingerprint: String,
    pub amount_idr: i64,
    pub payer_name: Option<String>,
    pub bank_rrn_ref: Option<String>,
    pub raw_notification_text: String,
    pub received_at: DateTime<Utc>,
    pub matched_invoice_id: Option<Uuid>,
    pub reconciliation_status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct UpgradedUserRow {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub status: String,
    pub tier: String,
    pub subscription_plan_id: Option<String>,
    pub subscription_expires_at: Option<DateTime<Utc>>,
    pub active_plan_code: Option<String>,
}
