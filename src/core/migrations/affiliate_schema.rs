/// Skema tabel Sistem Kode Referral & Afiliasi 20% (Double-Entry Ledger & Penarikan Manual 1x24 Jam Kerja).
pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
        BEGIN
            CREATE TYPE affiliate_ledger_entry_kind AS ENUM (
                'COMMISSION_CREDIT',
                'WITHDRAWAL_HOLD',
                'WITHDRAWAL_SETTLED',
                'WITHDRAWAL_REFUND_REJECTED',
                'WITHDRAWAL_REFUND_CANCELLED',
                'ADMIN_MANUAL_ADJUSTMENT'
            );
        EXCEPTION WHEN duplicate_object THEN NULL; END $$"#,
        r#"DO $$
        BEGIN
            CREATE TYPE affiliate_withdrawal_status AS ENUM (
                'PENDING_ADMIN_REVIEW',
                'PROCESSING_BY_ADMIN',
                'COMPLETED_TRANSFERRED',
                'REJECTED_REFUNDED',
                'CANCELLED_BY_USER'
            );
        EXCEPTION WHEN duplicate_object THEN NULL; END $$"#,
        r#"DO $$
        BEGIN
            CREATE TYPE payout_channel_kind AS ENUM (
                'BANK_MANDIRI', 'BANK_BCA', 'BANK_BRI', 'BANK_BNI',
                'BANK_BSI', 'BANK_SEABANK', 'BANK_JAGO',
                'EWALLET_GOPAY', 'EWALLET_DANA', 'EWALLET_OVO',
                'EWALLET_SHOPEEPAY', 'OTHER_BANK'
            );
        EXCEPTION WHEN duplicate_object THEN NULL; END $$"#,
        r#"CREATE TABLE IF NOT EXISTS affiliate_program_config (
            config_id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (config_id = 1),
            is_program_active BOOLEAN NOT NULL DEFAULT TRUE,
            buyer_discount_bps INT NOT NULL DEFAULT 1000 CHECK (buyer_discount_bps BETWEEN 0 AND 5000),
            affiliate_commission_bps INT NOT NULL DEFAULT 2000 CHECK (affiliate_commission_bps BETWEEN 100 AND 5000),
            commission_basis VARCHAR(25) NOT NULL DEFAULT 'BASE_PRICE',
            round_commission_to_nearest_idr INT NOT NULL DEFAULT 1000,
            min_withdrawal_idr BIGINT NOT NULL DEFAULT 100000 CHECK (min_withdrawal_idr >= 10000),
            max_withdrawal_per_day_idr BIGINT NOT NULL DEFAULT 5000000,
            withdrawal_admin_fee_idr BIGINT NOT NULL DEFAULT 0,
            first_purchase_only BOOLEAN NOT NULL DEFAULT FALSE,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "INSERT INTO affiliate_program_config (config_id) VALUES (1) ON CONFLICT (config_id) DO NOTHING",
        "ALTER TABLE affiliate_program_config ADD COLUMN IF NOT EXISTS allow_free_tier_affiliates BOOLEAN NOT NULL DEFAULT FALSE",
        r#"CREATE TABLE IF NOT EXISTS affiliate_wallets (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            affiliate_code VARCHAR(24) NOT NULL,
            is_custom_code_claimed BOOLEAN NOT NULL DEFAULT FALSE,
            is_affiliate_active BOOLEAN NOT NULL DEFAULT TRUE,
            available_balance_idr BIGINT NOT NULL DEFAULT 0 CHECK (available_balance_idr >= 0),
            locked_withdrawal_idr BIGINT NOT NULL DEFAULT 0 CHECK (locked_withdrawal_idr >= 0),
            total_earned_idr BIGINT NOT NULL DEFAULT 0 CHECK (total_earned_idr >= 0),
            total_withdrawn_idr BIGINT NOT NULL DEFAULT 0 CHECK (total_withdrawn_idr >= 0),
            total_successful_referrals INT NOT NULL DEFAULT 0,
            total_code_uses_count INT NOT NULL DEFAULT 0,
            saved_payout_channel payout_channel_kind,
            saved_account_number VARCHAR(64),
            saved_account_holder_name VARCHAR(120),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE UNIQUE INDEX IF NOT EXISTS uq_affiliate_wallets_code_upper ON affiliate_wallets (UPPER(affiliate_code))",
        r#"ALTER TABLE qris_subscription_invoices
            ADD COLUMN IF NOT EXISTS applied_referral_code VARCHAR(24),
            ADD COLUMN IF NOT EXISTS referrer_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            ADD COLUMN IF NOT EXISTS buyer_discount_idr BIGINT NOT NULL DEFAULT 0,
            ADD COLUMN IF NOT EXISTS affiliate_commission_bps INT NOT NULL DEFAULT 0,
            ADD COLUMN IF NOT EXISTS affiliate_commission_idr BIGINT NOT NULL DEFAULT 0,
            ADD COLUMN IF NOT EXISTS is_commission_credited BOOLEAN NOT NULL DEFAULT FALSE"#,
        "CREATE INDEX IF NOT EXISTS idx_qris_invoices_referrer ON qris_subscription_invoices (referrer_user_id, created_at DESC) WHERE referrer_user_id IS NOT NULL",
        r#"CREATE TABLE IF NOT EXISTS affiliate_referral_conversions (
            id UUID PRIMARY KEY,
            referrer_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            buyer_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            source_invoice_id UUID NOT NULL UNIQUE REFERENCES qris_subscription_invoices(id) ON DELETE CASCADE,
            affiliate_code_used VARCHAR(24) NOT NULL,
            plan_code subscription_plan_code NOT NULL,
            plan_base_price_idr BIGINT NOT NULL,
            buyer_discount_idr BIGINT NOT NULL,
            buyer_net_paid_idr BIGINT NOT NULL,
            commission_bps INT NOT NULL,
            commission_earned_idr BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT chk_no_self_referral CHECK (referrer_user_id <> buyer_user_id)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_aff_conversions_referrer ON affiliate_referral_conversions (referrer_user_id, created_at DESC)",
        r#"CREATE TABLE IF NOT EXISTS affiliate_withdrawal_requests (
            id UUID PRIMARY KEY,
            ticket_number VARCHAR(32) NOT NULL UNIQUE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            requested_amount_idr BIGINT NOT NULL CHECK (requested_amount_idr >= 100000),
            admin_fee_idr BIGINT NOT NULL DEFAULT 0,
            net_transfer_amount_idr BIGINT NOT NULL CHECK (net_transfer_amount_idr > 0),
            payout_channel payout_channel_kind NOT NULL,
            bank_or_wallet_label VARCHAR(80) NOT NULL,
            account_number VARCHAR(64) NOT NULL,
            account_holder_name VARCHAR(120) NOT NULL,
            user_notes VARCHAR(200),
            status affiliate_withdrawal_status NOT NULL DEFAULT 'PENDING_ADMIN_REVIEW',
            sla_deadline_at TIMESTAMPTZ NOT NULL,
            reviewed_by_admin_id UUID REFERENCES users(id) ON DELETE SET NULL,
            admin_transfer_ref_code VARCHAR(100),
            transfer_proof_cloudinary_url TEXT,
            transfer_proof_cloudinary_public_id TEXT,
            admin_notes TEXT,
            rejection_reason TEXT,
            processed_at TIMESTAMPTZ,
            completed_or_rejected_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_aff_withdrawals_user_history ON affiliate_withdrawal_requests (user_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_aff_withdrawals_admin_queue ON affiliate_withdrawal_requests (status, created_at ASC) WHERE status IN ('PENDING_ADMIN_REVIEW', 'PROCESSING_BY_ADMIN')",
        r#"CREATE TABLE IF NOT EXISTS affiliate_ledger_entries (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            idempotency_key VARCHAR(140) NOT NULL UNIQUE,
            entry_kind affiliate_ledger_entry_kind NOT NULL,
            available_delta_idr BIGINT NOT NULL,
            locked_delta_idr BIGINT NOT NULL DEFAULT 0,
            available_balance_after_idr BIGINT NOT NULL CHECK (available_balance_after_idr >= 0),
            locked_balance_after_idr BIGINT NOT NULL CHECK (locked_balance_after_idr >= 0),
            reference_invoice_id UUID REFERENCES qris_subscription_invoices(id) ON DELETE SET NULL,
            reference_withdrawal_id UUID REFERENCES affiliate_withdrawal_requests(id) ON DELETE SET NULL,
            actor_admin_id UUID REFERENCES users(id) ON DELETE SET NULL,
            description TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_aff_ledger_user_timeline ON affiliate_ledger_entries (user_id, id DESC)",
    ]
}
