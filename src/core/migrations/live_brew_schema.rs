pub fn statements() -> &'static [&'static str] {
    &[
        r#"
        CREATE TABLE IF NOT EXISTS business_brew_tables (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            store_id UUID REFERENCES business_store_configs(id) ON DELETE CASCADE,
            owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            table_number VARCHAR(30) NOT NULL,
            static_address_code VARCHAR(16) NOT NULL UNIQUE,
            active_brew_session_id UUID,
            qr_scans_count BIGINT NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT uq_owner_table_number UNIQUE (owner_user_id, table_number)
        );
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_brew_tables_static_code ON business_brew_tables (static_address_code);"#,
        r#"
        DO $$ BEGIN
            CREATE TYPE live_brew_stage AS ENUM (
                'QUEUED_ASSIGNED', 'GRINDING_DOSE', 'FILTER_RINSE_PREHEAT',
                'BLOOMING_DEGASSING', 'ACTIVE_POURING', 'DRAWDOWN_SWIRL',
                'READY_TO_SERVE', 'COMPLETED_ARCHIVED'
            );
        EXCEPTION WHEN duplicate_object THEN NULL;
        END $$;
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS live_brew_sessions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            seq_num BIGSERIAL UNIQUE,
            dynamic_address_code VARCHAR(16) NOT NULL UNIQUE,
            store_id UUID REFERENCES business_store_configs(id) ON DELETE SET NULL,
            table_id UUID REFERENCES business_brew_tables(id) ON DELETE SET NULL,
            pos_order_id UUID REFERENCES pos_orders(id) ON DELETE SET NULL,
            barista_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            barista_display_name VARCHAR(80) NOT NULL DEFAULT 'Barista Coffee Labs',
            customer_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            customer_display_name VARCHAR(80) NOT NULL DEFAULT 'Pelanggan Spesial',
            recipe_id UUID,
            recipe_title VARCHAR(140) NOT NULL,
            brew_method VARCHAR(40) NOT NULL DEFAULT 'V60',
            bean_name VARCHAR(120) NOT NULL DEFAULT 'Malabar Wine Reserve',
            roastery_name VARCHAR(120) NOT NULL DEFAULT 'Coffee Labs Slow Bar',
            bean_origin VARCHAR(120) NOT NULL DEFAULT 'Jawa Barat, 1400-1800 MDPL',
            bean_process VARCHAR(60) NOT NULL DEFAULT 'Wine Process',
            tasting_notes JSONB NOT NULL DEFAULT '[]'::jsonb,
            target_dose_g REAL NOT NULL DEFAULT 15.0,
            target_water_ml REAL NOT NULL DEFAULT 225.0,
            target_temp_c REAL NOT NULL DEFAULT 91.0,
            secondary_temp_c REAL,
            target_duration_sec INT NOT NULL DEFAULT 145,
            recipe_steps_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            current_stage live_brew_stage NOT NULL DEFAULT 'QUEUED_ASSIGNED',
            current_step_index SMALLINT NOT NULL DEFAULT 0,
            elapsed_sec INT NOT NULL DEFAULT 0,
            current_weight_g REAL NOT NULL DEFAULT 0.0,
            current_flow_rate_ml_s REAL NOT NULL DEFAULT 0.0,
            current_temp_c REAL NOT NULL DEFAULT 91.0,
            is_video_stream_active BOOLEAN NOT NULL DEFAULT FALSE,
            webrtc_whep_endpoint TEXT,
            recorded_telemetry_curve JSONB NOT NULL DEFAULT '[]'::jsonb,
            final_accuracy_score_pct REAL,
            cheers_reactions_count INT NOT NULL DEFAULT 0,
            viewer_peak_count INT NOT NULL DEFAULT 0,
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_live_brew_dynamic_code ON live_brew_sessions (dynamic_address_code);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_live_brew_store_active ON live_brew_sessions (barista_user_id, current_stage, created_at DESC);"#,
        r#"DO $$ BEGIN
            CREATE TYPE live_brew_mirror_phase AS ENUM (
                'SEAT_IDLE_BRANDED', 'POS_ORDER_QUEUED', 'BREWER_SELECTING_RECIPE',
                'BREWER_SELECTING_BEAN', 'PRE_BREW_CALIBRATED', 'BREWING_ACTIVE',
                'BREWING_PAUSED', 'SERVING_READY_PASSPORT', 'ARCHIVED'
            );
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;"#,
        r#"DO $$ BEGIN
            CREATE TYPE live_timer_status AS ENUM ('IDLE', 'RUNNING', 'PAUSED', 'FINISHED');
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;"#,
        r#"DO $$ BEGIN
            CREATE TYPE cafe_seat_kind AS ENUM ('MEJA', 'KURSI_BAR', 'VIP_ROOM', 'OUTDOOR', 'TAKEAWAY_COUNTER');
        EXCEPTION WHEN duplicate_object THEN NULL; END $$;"#,
        r#"CREATE TABLE IF NOT EXISTS business_livebrew_branding (
            store_id UUID PRIMARY KEY REFERENCES business_store_configs(id) ON DELETE CASCADE,
            owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            cafe_slug VARCHAR(64) NOT NULL UNIQUE,
            cafe_display_name VARCHAR(120) NOT NULL,
            logo_cloudinary_url TEXT,
            logo_cloudinary_public_id TEXT,
            primary_color_hex VARCHAR(10) NOT NULL DEFAULT '#12100E',
            accent_color_hex VARCHAR(10) NOT NULL DEFAULT '#D4A373',
            welcome_message VARCHAR(240) NOT NULL DEFAULT 'Selamat menikmati seduhan segar kami! Pantau proses seduhan Anda secara live di sini.',
            wifi_ssid VARCHAR(80),
            wifi_password VARCHAR(80),
            instagram_handle VARCHAR(64),
            show_pos_queue_position BOOLEAN NOT NULL DEFAULT TRUE,
            show_live_bean_details BOOLEAN NOT NULL DEFAULT TRUE,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );"#,
        r#"CREATE TABLE IF NOT EXISTS business_cafe_seats (
            id UUID PRIMARY KEY,
            store_id UUID NOT NULL REFERENCES business_livebrew_branding(store_id) ON DELETE CASCADE,
            seat_kind cafe_seat_kind NOT NULL DEFAULT 'MEJA',
            seat_label VARCHAR(40) NOT NULL,
            seat_slug VARCHAR(48) NOT NULL,
            zone_area VARCHAR(60) NOT NULL DEFAULT 'Main Area',
            capacity_persons SMALLINT NOT NULL DEFAULT 2,
            numeric_fallback_code VARCHAR(16) NOT NULL UNIQUE,
            active_live_session_id UUID,
            active_pos_order_id UUID,
            active_pos_order_number VARCHAR(32),
            active_pos_order_status VARCHAR(40),
            active_pos_items_summary TEXT,
            active_pos_customer_name VARCHAR(80),
            qr_scan_count BIGINT NOT NULL DEFAULT 0,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT uq_store_seat_slug UNIQUE (store_id, seat_slug)
        );"#,
        r#"CREATE INDEX IF NOT EXISTS idx_cafe_seats_store_zone ON business_cafe_seats (store_id, zone_area, seat_label);"#,
        r#"CREATE TABLE IF NOT EXISTS live_brew_real_sessions (
            id UUID PRIMARY KEY,
            seq_num BIGSERIAL UNIQUE,
            share_code VARCHAR(16) NOT NULL UNIQUE,
            is_business_session BOOLEAN NOT NULL DEFAULT FALSE,
            store_id UUID REFERENCES business_livebrew_branding(store_id) ON DELETE SET NULL,
            seat_id UUID REFERENCES business_cafe_seats(id) ON DELETE SET NULL,
            pos_order_id UUID,
            pos_order_number VARCHAR(32),
            pos_order_status VARCHAR(40),
            brewer_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            brewer_display_name VARCHAR(80) NOT NULL,
            brewer_avatar_url TEXT,
            customer_name VARCHAR(80) NOT NULL DEFAULT 'Tamu Coffee Labs',
            phase live_brew_mirror_phase NOT NULL DEFAULT 'BREWER_SELECTING_RECIPE',
            state_version BIGINT NOT NULL DEFAULT 1,
            selected_method_id VARCHAR(50),
            selected_method_name VARCHAR(80),
            selected_recipe_id UUID,
            selected_recipe_title VARCHAR(140),
            recipe_snapshot_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            selected_bean_id UUID,
            bean_snapshot_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            timer_status live_timer_status NOT NULL DEFAULT 'IDLE',
            timer_accumulated_ms BIGINT NOT NULL DEFAULT 0,
            timer_anchor_server_ms BIGINT,
            current_step_index SMALLINT NOT NULL DEFAULT 0,
            current_weight_g REAL NOT NULL DEFAULT 0.0,
            current_flow_rate_ml_s REAL NOT NULL DEFAULT 0.0,
            current_temp_c REAL NOT NULL DEFAULT 91.0,
            telemetry_curve_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            cheers_count INT NOT NULL DEFAULT 0,
            active_viewers_count INT NOT NULL DEFAULT 0,
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );"#,
        r#"CREATE INDEX IF NOT EXISTS idx_live_real_sessions_code ON live_brew_real_sessions (share_code);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_live_real_sessions_seat_active ON live_brew_real_sessions (seat_id, created_at DESC) WHERE phase <> 'ARCHIVED';"#,
        r#"CREATE INDEX IF NOT EXISTS idx_live_real_sessions_pos_order ON live_brew_real_sessions (pos_order_id) WHERE pos_order_id IS NOT NULL;"#,
    ]
}
