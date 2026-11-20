use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::qris_billing::models::{QrisInvoiceRow, QrisMutationRow, SubscriptionPlanRow};

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionPlanDto {
    pub plan_code: String,
    pub target_tier: String,
    pub display_name: String,
    pub duration_days: i32,
    pub base_price_idr: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_price_idr: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub promo_badge_text: Option<String>,
    pub is_highlighted: bool,
    pub features: Vec<String>,
    pub is_active: bool,
}

impl From<SubscriptionPlanRow> for SubscriptionPlanDto {
    fn from(r: SubscriptionPlanRow) -> Self {
        let features = r
            .features_json
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            plan_code: r.plan_code,
            target_tier: r.target_tier,
            display_name: r.display_name,
            duration_days: r.duration_days,
            base_price_idr: r.base_price_idr,
            original_price_idr: r.original_price_idr,
            promo_badge_text: r.promo_badge_text,
            is_highlighted: r.is_highlighted,
            features,
            is_active: r.is_active,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateQrisCheckoutDto {
    pub plan_code: String,
    pub referral_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DynamicQrisInvoiceDto {
    pub invoice_id: Uuid,
    pub invoice_number: String,
    pub plan_code: String,
    pub target_tier: String,
    pub duration_days: i32,
    pub base_amount_idr: i64,
    pub buyer_discount_idr: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_referral_code: Option<String>,
    pub affiliate_commission_idr: i64,
    pub unique_code_idr: i16,
    pub total_payable_idr: i64,
    pub dynamic_qris_payload: String,
    pub merchant_name: String,
    pub acquirer_code: String,
    pub nmid: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub manual_proof_cloudinary_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fresh_access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_expires_at: Option<DateTime<Utc>>,
}

impl DynamicQrisInvoiceDto {
    pub fn from_row(r: QrisInvoiceRow, nmid: String) -> Self {
        Self {
            invoice_id: r.id,
            invoice_number: r.invoice_number,
            plan_code: r.plan_code,
            target_tier: r.target_tier,
            duration_days: r.duration_days,
            base_amount_idr: r.base_amount_idr,
            buyer_discount_idr: r.buyer_discount_idr,
            applied_referral_code: r.applied_referral_code,
            affiliate_commission_idr: r.affiliate_commission_idr,
            unique_code_idr: r.unique_code_idr,
            total_payable_idr: r.total_payable_idr,
            dynamic_qris_payload: r.dynamic_qris_payload,
            merchant_name: r.merchant_name,
            acquirer_code: r.acquirer_code,
            nmid,
            status: r.status,
            expires_at: r.expires_at,
            paid_at: r.paid_at,
            manual_proof_cloudinary_url: r.manual_proof_cloudinary_url,
            fresh_access_token: None,
            subscription_expires_at: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BridgeIngestPayloadDto {
    pub package_name: Option<String>,
    pub title: Option<String>,
    pub raw_notification_text: String,
    pub amount_idr: Option<i64>,
    pub payer_name: Option<String>,
    pub bank_rrn_ref: Option<String>,
    pub received_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BridgeIngestResponseDto {
    pub mutation_id: Uuid,
    pub reconciliation_status: String,
    pub parsed_amount_idr: i64,
    pub matched_invoice_id: Option<Uuid>,
    pub upgraded_tier: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManualProofUploadDto {
    pub proof_cloudinary_url: String,
    #[allow(dead_code)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdminTestQrisDto {
    pub amount_idr: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTestQrisResponseDto {
    pub merchant_name: String,
    pub merchant_city: String,
    pub acquirer_code: String,
    pub nmid: String,
    pub amount_idr: i64,
    pub dynamic_qris_payload: String,
    pub crc_valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnmatchedMutationDto {
    pub id: Uuid,
    pub source_channel: String,
    pub amount_idr: i64,
    pub payer_name: Option<String>,
    pub bank_rrn_ref: Option<String>,
    pub raw_notification_text: String,
    pub received_at: DateTime<Utc>,
    pub reconciliation_status: String,
    pub matched_invoice_id: Option<Uuid>,
}

impl From<QrisMutationRow> for UnmatchedMutationDto {
    fn from(m: QrisMutationRow) -> Self {
        Self {
            id: m.id,
            source_channel: m.source_channel,
            amount_idr: m.amount_idr,
            payer_name: m.payer_name,
            bank_rrn_ref: m.bank_rrn_ref,
            raw_notification_text: m.raw_notification_text,
            received_at: m.received_at,
            reconciliation_status: m.reconciliation_status,
            matched_invoice_id: m.matched_invoice_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QrisWsFrame {
    InvoiceStatus {
        invoice_id: Uuid,
        status: String,
        expires_at: DateTime<Utc>,
    },
    PaymentConfirmed {
        invoice_id: Uuid,
        invoice_number: String,
        new_tier: String,
        plan_code: String,
        subscription_expires_at: DateTime<Utc>,
        fresh_access_token: String,
    },
    Expired {
        invoice_id: Uuid,
    },
}
