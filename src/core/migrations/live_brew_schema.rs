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
    ]
}
