use chrono::{Datelike, TimeZone, Utc, Weekday};
use chrono_tz::Asia::Jakarta;
use coffe_labs_backend::modules::affiliate::{
    code_generator::{
        generate_default_affiliate_code, generate_withdrawal_ticket, is_pro_eligible_tier,
        validate_and_normalize_custom_code,
    },
    math_and_sla::{compute_business_day_24h_sla, compute_checkout_affiliate_pricing},
    models::AffiliateConfigRow,
    services::withdrawal_payout_service::normalize_payout_channel,
};

fn default_cfg(round_step: i32, basis: &str) -> AffiliateConfigRow {
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
    }
}

#[test]
fn test_fintech_basis_points_pricing_zero_float_drift() {
    // Pro Plan Rp 99.000 -> Diskon 10% (Rp 9.900 -> Rp 89.100), Komisi 20% = Rp 19.800 (eksak) / Rp 20.000 (rounded 1000)
    let exact = compute_checkout_affiliate_pricing(99_000, &default_cfg(1, "BASE_PRICE"));
    assert_eq!(exact.buyer_discount_idr, 9_900);
    assert_eq!(exact.discounted_price_idr, 89_100);
    assert_eq!(exact.affiliate_commission_idr, 19_800);

    let rounded = compute_checkout_affiliate_pricing(99_000, &default_cfg(1000, "BASE_PRICE"));
    assert_eq!(rounded.affiliate_commission_idr, 20_000);

    // Business Plan Rp 249.000 -> Diskon 10% (Rp 24.900 -> Rp 224.100), Komisi 20% = Rp 50.000 (rounded 1000)
    let biz = compute_checkout_affiliate_pricing(249_000, &default_cfg(1000, "BASE_PRICE"));
    assert_eq!(biz.buyer_discount_idr, 24_900);
    assert_eq!(biz.discounted_price_idr, 224_100);
    assert_eq!(biz.affiliate_commission_idr, 50_000);

    // Mode NET_AFTER_DISCOUNT: 20% dari Rp 89.100 = Rp 17.820
    let net = compute_checkout_affiliate_pricing(99_000, &default_cfg(1, "NET_AFTER_DISCOUNT"));
    assert_eq!(net.affiliate_commission_idr, 17_820);
}

#[test]
fn test_business_hours_sla_weekend_rollover_wib() {
    // Jumat 20 Nov 2026 16:00 WIB (09:00 UTC) -> Senin 23 Nov 2026 16:00 WIB
    let fri = Utc.with_ymd_and_hms(2026, 11, 20, 9, 0, 0).single().expect("valid dt");
    let sla_fri = compute_business_day_24h_sla(fri).with_timezone(&Jakarta);
    assert_eq!(sla_fri.weekday(), Weekday::Mon);
    assert_eq!(sla_fri.day(), 23);

    // Sabtu 21 Nov 2026 10:00 WIB (03:00 UTC) -> Selasa 24 Nov 2026 09:00 WIB
    let sat = Utc.with_ymd_and_hms(2026, 11, 21, 3, 0, 0).single().expect("valid dt");
    let sla_sat = compute_business_day_24h_sla(sat).with_timezone(&Jakarta);
    assert_eq!(sla_sat.weekday(), Weekday::Tue);
    assert_eq!(sla_sat.day(), 24);
}

#[test]
fn test_pro_eligibility_code_generation_and_channels() {
    assert!(is_pro_eligible_tier("pro", "user"));
    assert!(is_pro_eligible_tier("business", "user"));
    assert!(is_pro_eligible_tier("trial", "admin"));
    assert!(!is_pro_eligible_tier("trial", "user"));

    let auto_code = generate_default_affiliate_code("risaha15");
    assert!(auto_code.starts_with("CL-"));
    assert!(auto_code.len() >= 9);

    assert_eq!(
        validate_and_normalize_custom_code("  baristabudi ").expect("valid vanity"),
        "BARISTABUDI"
    );
    assert!(validate_and_normalize_custom_code("ab").is_err());
    assert!(validate_and_normalize_custom_code("ADMIN").is_err());

    let ticket = generate_withdrawal_ticket(Utc::now());
    assert!(ticket.starts_with("WD-"));
    assert_eq!(normalize_payout_channel("bca"), Some(("BANK_BCA", "Bank BCA")));
    assert_eq!(normalize_payout_channel("gopay"), Some(("EWALLET_GOPAY", "GoPay")));
    assert_eq!(normalize_payout_channel("unknown_xyz"), None);
}
