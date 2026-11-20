-- Migration: Live Brew 2.0 — Zero-Simulation 1:1 Mirrored State Machine,
-- Custom Branded Cafe + Table/Chair Barcodes & Full Business POS Synchronization

DO $$ BEGIN
    CREATE TYPE live_brew_mirror_phase AS ENUM (
        'SEAT_IDLE_BRANDED',
        'POS_ORDER_QUEUED',
        'BREWER_SELECTING_RECIPE',
        'BREWER_SELECTING_BEAN',
        'PRE_BREW_CALIBRATED',
        'BREWING_ACTIVE',
        'BREWING_PAUSED',
        'SERVING_READY_PASSPORT',
        'ARCHIVED'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE live_timer_status AS ENUM ('IDLE', 'RUNNING', 'PAUSED', 'FINISHED');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE cafe_seat_kind AS ENUM ('MEJA', 'KURSI_BAR', 'VIP_ROOM', 'OUTDOOR', 'TAKEAWAY_COUNTER');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS business_livebrew_branding (
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
);

CREATE TABLE IF NOT EXISTS business_cafe_seats (
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
);

CREATE INDEX IF NOT EXISTS idx_cafe_seats_store_zone
    ON business_cafe_seats (store_id, zone_area, seat_label);

CREATE TABLE IF NOT EXISTS live_brew_real_sessions (
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
);

CREATE INDEX IF NOT EXISTS idx_live_real_sessions_code
    ON live_brew_real_sessions (share_code);
CREATE INDEX IF NOT EXISTS idx_live_real_sessions_seat_active
    ON live_brew_real_sessions (seat_id, created_at DESC)
    WHERE phase <> 'ARCHIVED';
CREATE INDEX IF NOT EXISTS idx_live_real_sessions_pos_order
    ON live_brew_real_sessions (pos_order_id)
    WHERE pos_order_id IS NOT NULL;
