-- Upgrade katalog lisensi QRIS Dinamis menjadi Lifetime Promo Akhir 2026:
-- PRO_LIFETIME      = Rp 99.000  (Sekali Bayar • Selamanya)
-- BUSINESS_LIFETIME = Rp 249.000 (Sekali Bayar • Selamanya)

ALTER TYPE subscription_plan_code ADD VALUE IF NOT EXISTS 'PRO_LIFETIME';
ALTER TYPE subscription_plan_code ADD VALUE IF NOT EXISTS 'BUSINESS_LIFETIME';
