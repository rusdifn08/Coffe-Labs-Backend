pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
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
        $$"#,
        r#"DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'subscription_plan_code') THEN
                CREATE TYPE subscription_plan_code AS ENUM (
                    'PRO_MONTHLY',
                    'PRO_YEARLY',
                    'BUSINESS_MONTHLY',
                    'BUSINESS_YEARLY',
                    'PRO_LIFETIME',
                    'BUSINESS_LIFETIME'
                );
            END IF;
        END
        $$"#,
        "ALTER TYPE subscription_plan_code ADD VALUE IF NOT EXISTS 'PRO_LIFETIME'",
        "ALTER TYPE subscription_plan_code ADD VALUE IF NOT EXISTS 'BUSINESS_LIFETIME'",
        r#"CREATE TABLE IF NOT EXISTS subscription_plan_catalog (
            plan_code subscription_plan_code PRIMARY KEY,
            target_tier VARCHAR(20) NOT NULL,
            display_name VARCHAR(80) NOT NULL,
            duration_days INT NOT NULL,
            base_price_idr BIGINT NOT NULL,
            features_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"INSERT INTO subscription_plan_catalog (
            plan_code, target_tier, display_name, duration_days, base_price_idr, features_json, is_active
        ) VALUES
            (
                'PRO_LIFETIME',
                'pro',
                'Coffee Labs Pro Lifetime (Promo Akhir 2026)',
                36500,
                99000,
                '["Sekali Bayar • Akses Penuh Selamanya (Promo s/d 31 Des 2026)", "Buka Seluruh 500+ Resep Juara Dunia & Nusantara (18 Metode Seduh)", "Seduh Tanpa Batas & Alarm Latar Belakang Cold Brew", "Toolkit Barista Lengkap 21 Fitur (Smart Brew AI, Grinder Matrix, TDS)", "All 10 World Coffee E-Books, Akademi LMS & AI Bean Scanner", "Live Brew QR 2.0 & Bebas Iklan 100% Selamanya"]'::jsonb,
                TRUE
            ),
            (
                'BUSINESS_LIFETIME',
                'business',
                'Coffee Labs Business Lifetime (Promo Akhir 2026)',
                36500,
                249000,
                '["Sekali Bayar • Lisensi Bisnis Selamanya (Promo s/d 31 Des 2026)", "Seluruh Fitur Coffee Labs Pro (500+ Resep & Toolkit 21 Fitur)", "Business Roastery Studio: 1 Profil Roastery & Unlimited Lineup (+ AI Scan)", "POS Kasir Lengkap, Smart Bean Vault & Cetak Struk Bluetooth 58/80mm", "Strict AMOLED AOD Kiosk Mode Anti Burn-In & Kunci PIN", "Custom Branded Live Brew 2.0 & Cetak QR Meja/Kursi Kafe"]'::jsonb,
                TRUE
            )
        ON CONFLICT (plan_code) DO NOTHING"#,
        r#"CREATE TABLE IF NOT EXISTS qris_subscription_invoices (
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
        )"#,
        "CREATE UNIQUE INDEX IF NOT EXISTS uq_qris_active_pending_amount ON qris_subscription_invoices (total_payable_idr) WHERE status = 'PENDING'",
        "CREATE INDEX IF NOT EXISTS idx_qris_invoices_user_history ON qris_subscription_invoices (user_id, created_at DESC)",
        r#"CREATE TABLE IF NOT EXISTS qris_payment_mutations (
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
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_qris_mutations_unmatched ON qris_payment_mutations (amount_idr, received_at DESC) WHERE reconciliation_status = 'UNMATCHED'",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS subscription_expires_at TIMESTAMPTZ",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS active_plan_code subscription_plan_code",
    ]
}
