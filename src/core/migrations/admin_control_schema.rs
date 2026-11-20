pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'feature_lock_status_kind') THEN
                CREATE TYPE feature_lock_status_kind AS ENUM (
                    'UNLOCKED',
                    'MAINTENANCE',
                    'COMING_SOON',
                    'UPDATED'
                );
            END IF;
        END
        $$"#,
        r#"CREATE TABLE IF NOT EXISTS system_control_meta (
            singleton_id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (singleton_id = 1),
            config_version BIGINT NOT NULL DEFAULT 1,
            global_maintenance_mode BOOLEAN NOT NULL DEFAULT FALSE,
            global_maintenance_message TEXT,
            updated_by_admin_id UUID REFERENCES users(id) ON DELETE SET NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "INSERT INTO system_control_meta (singleton_id, config_version) VALUES (1, 1) ON CONFLICT (singleton_id) DO NOTHING",
        r#"CREATE TABLE IF NOT EXISTS toolkit_feature_controls (
            feature_key VARCHAR(50) PRIMARY KEY,
            display_name VARCHAR(100) NOT NULL,
            group_section VARCHAR(60) NOT NULL DEFAULT 'EKOSISTEM_KOMUNITAS',
            route_prefix VARCHAR(100) NOT NULL,
            api_path_prefixes JSONB NOT NULL DEFAULT '[]'::jsonb,
            display_order SMALLINT NOT NULL DEFAULT 10,
            is_locked BOOLEAN NOT NULL DEFAULT FALSE,
            lock_status feature_lock_status_kind NOT NULL DEFAULT 'UNLOCKED',
            custom_badge_label VARCHAR(40),
            lock_title VARCHAR(140) NOT NULL DEFAULT '',
            lock_message TEXT NOT NULL DEFAULT '',
            estimated_unlock_at TIMESTAMPTZ,
            allow_admin_bypass BOOLEAN NOT NULL DEFAULT TRUE,
            updated_by_admin_id UUID REFERENCES users(id) ON DELETE SET NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"INSERT INTO toolkit_feature_controls
            (feature_key, display_name, group_section, route_prefix, api_path_prefixes, display_order)
        VALUES
            ('GAME_BREW_3D',       'Game Brew V60',          'LEARNING_BREW',       '/toolkit/game-brew',   '["/api/v1/toolkit/game-brew"]'::jsonb, 1),
            ('EBOOK_DRM_LIBRARY',  'Pustaka E-Book DRM',     'LEARNING_BREW',       '/toolkit/ebooks',      '["/api/v1/toolkit/ebooks"]'::jsonb, 2),
            ('LMS_EBOOK_ACADEMY',  'Akademi Barista LMS',    'LEARNING_BREW',       '/toolkit/lms',         '["/api/v1/toolkit/lms"]'::jsonb, 3),
            ('BREW_DOCTOR',        'Brew Doctor & Kompas',   'LEARNING_BREW',       '/toolkit/brew-doctor', '["/api/v1/toolkit/brew-doctor"]'::jsonb, 4),
            ('MARKETPLACE_KOPI',   'Marketplace Kopi',       'EKOSISTEM_KOMUNITAS', '/marketplace',         '["/api/v1/marketplace"]'::jsonb, 5),
            ('COFFEE_ROASTERY',    'Coffee Roastery',        'EKOSISTEM_KOMUNITAS', '/roasteries',          '["/api/v1/roasteries"]'::jsonb, 6),
            ('BEANS_RADAR',        'Beans Radar',            'EKOSISTEM_KOMUNITAS', '/bean-radar',          '["/api/v1/beans-radar", "/api/v1/radar"]'::jsonb, 7),
            ('CAFE_RADAR',         'Cafe Radar',             'EKOSISTEM_KOMUNITAS', '/cafe-radar',          '["/api/v1/radar/cafe"]'::jsonb, 8),
            ('JURNAL_BREW',        'Jurnal Brew',            'EKOSISTEM_KOMUNITAS', '/brew-journal',        '["/api/v1/brew-logs"]'::jsonb, 9),
            ('SOCIAL_COFFEE',      'Social Coffee',          'EKOSISTEM_KOMUNITAS', '/social',              '["/api/v1/social", "/api/v1/chat", "/api/v1/communities"]'::jsonb, 10),
            ('FAVORITE_RECIPES',   'Favorite',               'EKOSISTEM_KOMUNITAS', '/favorites',           '["/api/v1/favorites"]'::jsonb, 11),
            ('LIVE_BREW_QR',       'Live Brew QR',           'EKOSISTEM_KOMUNITAS', '/livebrew',            '["/api/v1/live-brew"]'::jsonb, 12),
            ('SMART_BREW_AI',      'Smart Brew AI',          'LAB_EKSTRAKSI_AI',    '/toolkit/smart-brew',  '["/api/v1/recommendations"]'::jsonb, 13),
            ('AI_BEAN_SCANNER',    'Studio AI & Scanner',    'LAB_EKSTRAKSI_AI',    '/in-app-trainer',      '["/api/v1/bean-scanner", "/api/v1/vision-dataset"]'::jsonb, 14),
            ('DIAL_IN_ASSISTANT',  'Asisten Dial-In',        'LAB_EKSTRAKSI_AI',    '/dial-in',             '[]'::jsonb, 15),
            ('GRINDER_MATRIX',     'Matriks Grinder',        'LAB_EKSTRAKSI_AI',    '/toolkit/grinder',     '[]'::jsonb, 16),
            ('FOUR_SIX_CALC',      'Kalkulator 4:6',         'LAB_EKSTRAKSI_AI',    '/toolkit/four-six',    '[]'::jsonb, 17),
            ('TDS_EXTRACTION',     'Ekstraksi & TDS',        'LAB_EKSTRAKSI_AI',    '/toolkit/tds',         '[]'::jsonb, 18),
            ('WATER_CHEMISTRY',    'Kimia Air Seduh',        'SAINS_KOPI',          '/toolkit/water',       '[]'::jsonb, 19),
            ('BEAN_FRESHNESS',     'Kesegaran Biji',         'SAINS_KOPI',          '/toolkit/freshness',   '[]'::jsonb, 20),
            ('SMART_SCALE',        'Smart Scale Brew',       'SAINS_KOPI',          '/smart-scale',         '[]'::jsonb, 21),
            ('CUPPING_FLAVOR',     'Cupping Flavour',        'SAINS_KOPI',          '/toolkit/cupping',     '[]'::jsonb, 22),
            ('AFFILIATE_PROGRAM',  'Program Afiliasi 20%',   'MONETISASI_BISNIS',   '/affiliate',           '["/api/v1/affiliate"]'::jsonb, 23),
            ('BUSINESS_POS_KIOSK', 'POS Kasir & AOD Kiosk',  'MONETISASI_BISNIS',   '/business',            '["/api/v1/business"]'::jsonb, 24)
        ON CONFLICT (feature_key) DO NOTHING"#,
        "ALTER TABLE subscription_plan_catalog ADD COLUMN IF NOT EXISTS original_price_idr BIGINT",
        "ALTER TABLE subscription_plan_catalog ADD COLUMN IF NOT EXISTS promo_badge_text VARCHAR(40)",
        "ALTER TABLE subscription_plan_catalog ADD COLUMN IF NOT EXISTS is_highlighted BOOLEAN NOT NULL DEFAULT FALSE",
        "ALTER TABLE subscription_plan_catalog ADD COLUMN IF NOT EXISTS updated_by_admin_id UUID REFERENCES users(id) ON DELETE SET NULL",
        r#"UPDATE subscription_plan_catalog
           SET original_price_idr = COALESCE(original_price_idr, 490000),
               promo_badge_text = COALESCE(promo_badge_text, 'POPULER'),
               is_highlighted = TRUE
           WHERE plan_code::text = 'PRO_LIFETIME' AND original_price_idr IS NULL"#,
        r#"UPDATE subscription_plan_catalog
           SET original_price_idr = COALESCE(original_price_idr, 1490000),
               promo_badge_text = COALESCE(promo_badge_text, 'KAFE & ROASTERY')
           WHERE plan_code::text = 'BUSINESS_LIFETIME' AND original_price_idr IS NULL"#,
        r#"INSERT INTO subscription_plan_catalog (
            plan_code, target_tier, display_name, duration_days, base_price_idr,
            original_price_idr, promo_badge_text, is_highlighted, features_json, is_active
        ) VALUES
            ('PRO_MONTHLY', 'pro', 'Coffee Labs Pro (Bulanan)', 30, 49000, 79000, 'FLEKSIBEL', FALSE,
             '["Akses Penuh 500+ Resep Dunia & Nusantara", "Toolkit Barista Lengkap 22 Fitur", "Akademi LMS & E-Book DRM", "Otomatis Partner Afiliasi 20%"]'::jsonb, FALSE),
            ('PRO_YEARLY', 'pro', 'Coffee Labs Pro (Tahunan)', 365, 399000, 588000, 'HEMAT 32%', FALSE,
             '["Akses Penuh 1 Tahun Semua Fitur Pro", "500+ Resep Dunia & Smart Brew AI", "Akademi LMS & 12 E-Book DRM", "Partner Afiliasi Komisi 20%"]'::jsonb, FALSE),
            ('BUSINESS_MONTHLY', 'business', 'Coffee Labs Business (Bulanan)', 30, 149000, 249000, 'UNTUK KAFE', FALSE,
             '["Seluruh Fitur Coffee Labs Pro", "POS Kasir, Smart Bean Vault & Struk Bluetooth", "Business Roastery Studio & Lineup Publik", "Live Brew QR 2.0 & AMOLED Kiosk Mode"]'::jsonb, FALSE),
            ('BUSINESS_YEARLY', 'business', 'Coffee Labs Business (Tahunan)', 365, 1190000, 1788000, 'HEMAT 33%', FALSE,
             '["Lisensi Komersial Kafe & Roastery 1 Tahun", "Seluruh Fitur Pro + POS + AOD Kiosk", "Business Roastery Studio & QR Meja", "Prioritas Dukungan Teknis & Update"]'::jsonb, FALSE)
        ON CONFLICT (plan_code) DO NOTHING"#,
        r#"CREATE TABLE IF NOT EXISTS admin_control_audit_logs (
            id UUID PRIMARY KEY,
            admin_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
            admin_email VARCHAR(160) NOT NULL,
            action_category VARCHAR(40) NOT NULL,
            target_key VARCHAR(80) NOT NULL,
            previous_state_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            new_state_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            reason_note VARCHAR(240),
            config_version_after BIGINT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_admin_control_audit_recent ON admin_control_audit_logs (created_at DESC, id DESC)",
    ]
}
