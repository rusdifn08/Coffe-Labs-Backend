-- Dynamic QRIS Zero-Gateway Subscription & Auto-Reconciliation Schema
-- Merchant: Coffeelabs (Bank Mandiri Livin' Merchant Acquirer 93600008, NMID ID1023277752888)

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'qris_invoice_status') THEN
        CREATE TYPE qris_invoice_status AS ENUM (
            'PENDING',
            'PAID',
            'EXPIRED',
            'CANCELLED',
            'MANUAL_REVIEW'
        );
    END IF;
END
$$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'subscription_plan_code') THEN
        CREATE TYPE subscription_plan_code AS ENUM (
            'PRO_MONTHLY',
            'PRO_YEARLY',
            'BUSINESS_MONTHLY',
            'BUSINESS_YEARLY'
        );
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS subscription_plan_catalog (
    plan_code subscription_plan_code PRIMARY KEY,
    target_tier VARCHAR(20) NOT NULL,
    display_name VARCHAR(80) NOT NULL,
    duration_days INT NOT NULL,
    base_price_idr BIGINT NOT NULL,
    features_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO subscription_plan_catalog (
    plan_code, target_tier, display_name, duration_days, base_price_idr, features_json
) VALUES
    (
        'PRO_MONTHLY',
        'pro',
        'Coffee Labs Pro (1 Bulan)',
        30,
        49000,
        '["Buka Seluruh 500+ Resep Juara Dunia & Nusantara", "Seduh Tanpa Batas & Alarm Cold Brew", "Toolkit Barista Lengkap 21 Fitur (Dial-In AI, Grinder Matrix, TDS)", "All 10 World Coffee E-Books & Akademi LMS", "AI Bean Scanner & Bebas Iklan 100%"]'::jsonb
    ),
    (
        'PRO_YEARLY',
        'pro',
        'Coffee Labs Pro (1 Tahun - Hemat 2 Bulan)',
        365,
        490000,
        '["Semua Fitur Pro Bulanan", "Hemat Rp98.000/tahun", "Priority AI Model & Recipe Updates"]'::jsonb
    ),
    (
        'BUSINESS_MONTHLY',
        'business',
        'Coffee Labs Business (1 Bulan)',
        30,
        149000,
        '["Seluruh Fitur Coffee Labs Pro", "Daftarkan 1 Profil Roastery & Unlimited Lineup Kopi (+ AI Scan)", "POS Kasir Lengkap & Smart Bean Vault", "Strict AMOLED AOD Kiosk Mode", "Custom Branded Live Brew 2.0 & Cetak QR Meja/Kursi + Bluetooth Thermal"]'::jsonb
    ),
    (
        'BUSINESS_YEARLY',
        'business',
        'Coffee Labs Business (1 Tahun - Hemat 2 Bulan)',
        365,
        1490000,
        '["Semua Fitur Business Bulanan", "Hemat Rp298.000/tahun", "Verified Roastery Partner Badge"]'::jsonb
    )
ON CONFLICT (plan_code) DO NOTHING;

CREATE TABLE IF NOT EXISTS qris_subscription_invoices (
    id UUID PRIMARY KEY,
    invoice_number VARCHAR(36) NOT NULL UNIQUE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_code subscription_plan_code NOT NULL REFERENCES subscription_plan_catalog(plan_code),
    target_tier VARCHAR(20) NOT NULL,
    duration_days INT NOT NULL,
    base_amount_idr BIGINT NOT NULL,
    unique_code_idr SMALLINT NOT NULL CHECK (unique_code_idr BETWEEN 1 AND 999),
    total_payable_idr BIGINT NOT NULL,
    dynamic_qris_payload TEXT NOT NULL,
    merchant_name VARCHAR(64) NOT NULL DEFAULT 'Coffeelabs',
    acquirer_code VARCHAR(20) NOT NULL DEFAULT '93600008',
    status qris_invoice_status NOT NULL DEFAULT 'PENDING',
    expires_at TIMESTAMPTZ NOT NULL,
    paid_at TIMESTAMPTZ,
    matched_mutation_id UUID,
    manual_proof_cloudinary_url TEXT,
    receipt_pdf_cloudinary_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_qris_active_pending_amount
    ON qris_subscription_invoices (total_payable_idr)
    WHERE status = 'PENDING';

CREATE INDEX IF NOT EXISTS idx_qris_invoices_user_history
    ON qris_subscription_invoices (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS qris_payment_mutations (
    id UUID PRIMARY KEY,
    source_channel VARCHAR(30) NOT NULL,
    source_fingerprint VARCHAR(128) NOT NULL UNIQUE,
    amount_idr BIGINT NOT NULL,
    payer_name VARCHAR(120),
    bank_rrn_ref VARCHAR(80),
    raw_notification_text TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL,
    matched_invoice_id UUID REFERENCES qris_subscription_invoices(id) ON DELETE SET NULL,
    reconciliation_status VARCHAR(30) NOT NULL DEFAULT 'UNMATCHED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_qris_mutations_unmatched
    ON qris_payment_mutations (amount_idr, received_at DESC)
    WHERE reconciliation_status = 'UNMATCHED';

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS subscription_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS active_plan_code subscription_plan_code;
