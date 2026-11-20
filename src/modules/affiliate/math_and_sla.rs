use chrono::{DateTime, Datelike, Duration, Utc, Weekday};
use chrono_tz::Asia::Jakarta;

use crate::modules::affiliate::models::AffiliateConfigRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AffiliatePricingBreakdown {
    pub base_price_idr: i64,
    pub buyer_discount_bps: i32,
    pub buyer_discount_idr: i64,
    pub discounted_price_idr: i64,
    pub affiliate_commission_bps: i32,
    pub affiliate_commission_idr: i64,
}

/// Pure-Integer Basis Points Fintech Pricing Calculator (Zero Floating-Point Drift).
/// 100 bps = 1.00%, 1000 bps = 10.00%, 2000 bps = 20.00%.
pub fn compute_checkout_affiliate_pricing(
    base_price_idr: i64,
    cfg: &AffiliateConfigRow,
) -> AffiliatePricingBreakdown {
    let base = base_price_idr.max(0);
    let disc_bps = i64::from(cfg.buyer_discount_bps.clamp(0, 5_000));
    let comm_bps = i64::from(cfg.affiliate_commission_bps.clamp(0, 5_000));

    let buyer_discount_idr = (base.saturating_mul(disc_bps)) / 10_000;
    let discounted_price_idr = base.saturating_sub(buyer_discount_idr).max(1_000);

    let comm_base = if cfg.commission_basis.eq_ignore_ascii_case("NET_AFTER_DISCOUNT") {
        discounted_price_idr
    } else {
        base
    };
    let raw_comm = (comm_base.saturating_mul(comm_bps)) / 10_000;

    let step = i64::from(cfg.round_commission_to_nearest_idr.max(1));
    let affiliate_commission_idr = if step > 1 {
        ((raw_comm + (step / 2)) / step) * step
    } else {
        raw_comm
    };

    AffiliatePricingBreakdown {
        base_price_idr: base,
        buyer_discount_bps: cfg.buyer_discount_bps,
        buyer_discount_idr,
        discounted_price_idr,
        affiliate_commission_bps: cfg.affiliate_commission_bps,
        affiliate_commission_idr,
    }
}

/// Menghitung tenggat waktu tepat 1x24 jam HARI KERJA (WIB `Asia/Jakarta`),
/// otomatis melompati hari Sabtu & Minggu agar SLA penarikan manual selalu akurat.
pub fn compute_business_day_24h_sla(now_utc: DateTime<Utc>) -> DateTime<Utc> {
    let local = now_utc.with_timezone(&Jakarta);
    let target_local = match local.weekday() {
        Weekday::Fri => local + Duration::days(3), // Jumat -> Senin (lompati Sabtu & Minggu)
        Weekday::Sat => {
            // Sabtu -> Selasa 09:00 WIB (1 hari kerja penuh sejak Senin pagi)
            let monday = local + Duration::days(2);
            monday
                .date_naive()
                .and_hms_opt(9, 0, 0)
                .and_then(|dt| dt.and_local_timezone(Jakarta).single())
                .map(|m| m + Duration::days(1))
                .unwrap_or(local + Duration::days(3))
        }
        Weekday::Sun => {
            // Minggu -> Selasa 09:00 WIB
            let monday = local + Duration::days(1);
            monday
                .date_naive()
                .and_hms_opt(9, 0, 0)
                .and_then(|dt| dt.and_local_timezone(Jakarta).single())
                .map(|m| m + Duration::days(1))
                .unwrap_or(local + Duration::days(2))
        }
        _ => local + Duration::hours(24),
    };
    target_local.with_timezone(&Utc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_cfg(round_step: i32, basis: &str) -> AffiliateConfigRow {
        AffiliateConfigRow {
            is_program_active: true,
            buyer_discount_bps: 1000,
            affiliate_commission_bps: 2000,
            commission_basis: basis.to_string(),
            round_commission_to_nearest_idr: round_step,
            min_withdrawal_idr: 100_000,
            max_withdrawal_per_day_idr: 5_000_000,
            withdrawal_admin_fee_idr: 0,
            first_purchase_only: false,
            allow_free_tier_affiliates: false,
        }
    }

    #[test]
    fn test_pro_99k_exact_and_rounded_20_percent_commission() {
        let exact = compute_checkout_affiliate_pricing(99_000, &sample_cfg(1, "BASE_PRICE"));
        assert_eq!(exact.buyer_discount_idr, 9_900);
        assert_eq!(exact.discounted_price_idr, 89_100);
        assert_eq!(exact.affiliate_commission_idr, 19_800);

        let rounded = compute_checkout_affiliate_pricing(99_000, &sample_cfg(1000, "BASE_PRICE"));
        assert_eq!(rounded.buyer_discount_idr, 9_900);
        assert_eq!(rounded.discounted_price_idr, 89_100);
        assert_eq!(rounded.affiliate_commission_idr, 20_000);

        let biz = compute_checkout_affiliate_pricing(249_000, &sample_cfg(1000, "BASE_PRICE"));
        assert_eq!(biz.buyer_discount_idr, 24_900);
        assert_eq!(biz.discounted_price_idr, 224_100);
        assert_eq!(biz.affiliate_commission_idr, 50_000);
    }

    #[test]
    fn test_business_day_24h_sla_skips_weekend_in_wib() {
        // Jumat 20 Nov 2026 pukul 16:00 WIB (= 09:00 UTC) -> Senin 23 Nov 2026 pukul 16:00 WIB
        let fri_utc = Utc.with_ymd_and_hms(2026, 11, 20, 9, 0, 0).single().unwrap();
        let sla = compute_business_day_24h_sla(fri_utc).with_timezone(&Jakarta);
        assert_eq!(sla.weekday(), Weekday::Mon);
        assert_eq!(sla.day(), 23);

        // Selasa 17 Nov 2026 pukul 14:00 WIB -> Rabu 18 Nov 2026 pukul 14:00 WIB
        let tue_utc = Utc.with_ymd_and_hms(2026, 11, 17, 7, 0, 0).single().unwrap();
        let sla_tue = compute_business_day_24h_sla(tue_utc).with_timezone(&Jakarta);
        assert_eq!(sla_tue.weekday(), Weekday::Wed);
        assert_eq!(sla_tue.day(), 18);
    }
}
