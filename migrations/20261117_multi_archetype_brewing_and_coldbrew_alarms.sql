-- Migration: 18-Method Multi-Archetype Execution & Persistent Cold Brew Alarms
-- Guarantees zero breakage for Hario V60 Percolation recipes while fixing catalog icons & enabling OS/Server alarms.

DO $$ BEGIN
    CREATE TYPE brew_execution_archetype AS ENUM (
        'PERCOLATION_POUR_OVER',
        'FLASH_CHILL_ICED_FILTER',
        'STEEP_AND_RELEASE_IMMERSION',
        'SUB_SECOND_PRESSURE_ESPRESSO',
        'THERMAL_CUE_PHASE_BREWER',
        'LONG_HORIZON_PERSISTENT_ALARM'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS brewing_methods_catalog (
    id SERIAL PRIMARY KEY,
    slug VARCHAR(48) NOT NULL UNIQUE,
    display_name VARCHAR(64) NOT NULL,
    display_name_id VARCHAR(64) NOT NULL,
    archetype brew_execution_archetype NOT NULL,
    icon_asset_path VARCHAR(160) NOT NULL,
    filter_key VARCHAR(48) NOT NULL,
    is_background_alarm_mode BOOLEAN NOT NULL DEFAULT FALSE,
    default_min_duration_sec INT NOT NULL DEFAULT 60,
    default_max_duration_sec INT NOT NULL DEFAULT 300,
    sort_order SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO brewing_methods_catalog
    (slug, display_name, display_name_id, archetype, icon_asset_path, filter_key, is_background_alarm_mode, default_min_duration_sec, default_max_duration_sec, sort_order)
VALUES
    ('aeropress', 'AeroPress', 'AeroPress', 'STEEP_AND_RELEASE_IMMERSION', 'assets/images/methods/aeropress.png', 'AeroPress', FALSE, 60, 600, 1),
    ('bee_house', 'Bee House', 'Bee House', 'PERCOLATION_POUR_OVER', 'assets/images/methods/bee_house.png', 'Bee House', FALSE, 120, 240, 2),
    ('bonmac', 'Bonmac Dripper', 'Bonmac Dripper', 'PERCOLATION_POUR_OVER', 'assets/images/methods/bonmac.png', 'Bonmac', FALSE, 120, 240, 3),
    ('chemex', 'Chemex', 'Chemex', 'PERCOLATION_POUR_OVER', 'assets/images/methods/chemex.png', 'Chemex', FALSE, 180, 360, 4),
    ('clever_dripper', 'Clever Dripper', 'Clever Dripper', 'STEEP_AND_RELEASE_IMMERSION', 'assets/images/methods/clever_dripper.png', 'Clever', FALSE, 120, 270, 5),
    ('cupping', 'Coffee Cupping', 'Cupping Kopi', 'STEEP_AND_RELEASE_IMMERSION', 'assets/images/methods/cupping.png', 'Cupping', FALSE, 600, 900, 6),
    ('drip_coffee_maker', 'Drip Coffee Maker', 'Mesin Drip Kopi', 'PERCOLATION_POUR_OVER', 'assets/images/methods/drip_coffee_maker.png', 'Drip Coffee Maker', FALSE, 180, 420, 7),
    ('espresso', 'Espresso', 'Espresso', 'SUB_SECOND_PRESSURE_ESPRESSO', 'assets/images/methods/espresso_machine.png', 'Espresso', FALSE, 18, 60, 8),
    ('french_press', 'French Press', 'French Press', 'STEEP_AND_RELEASE_IMMERSION', 'assets/images/methods/french_press.png', 'French Press', FALSE, 240, 540, 9),
    ('hario_v60', 'Hario V60', 'Hario V60', 'PERCOLATION_POUR_OVER', 'assets/images/methods/hario_v60.png', 'V60', FALSE, 105, 270, 10),
    ('kalita_wave', 'Kalita Wave', 'Kalita Wave', 'PERCOLATION_POUR_OVER', 'assets/images/methods/kalita_wave.png', 'Kalita Wave', FALSE, 130, 240, 11),
    ('moka_pot', 'Moka Pot', 'Moka Pot (Stovetop)', 'THERMAL_CUE_PHASE_BREWER', 'assets/images/methods/moka_pot.png', 'Moka Pot', FALSE, 120, 360, 12),
    ('syphon', 'Syphon', 'Syphon (Vakum)', 'THERMAL_CUE_PHASE_BREWER', 'assets/images/methods/syphon.png', 'Syphon', FALSE, 90, 240, 13),
    ('cold_brew', 'Cold Brew', 'Cold Brew & Drip', 'LONG_HORIZON_PERSISTENT_ALARM', 'assets/images/methods/cold_brew.png', 'Cold Brew', TRUE, 21600, 86400, 14),
    ('japanese_drip', 'Japanese Drip', 'Japanese Iced Drip', 'FLASH_CHILL_ICED_FILTER', 'assets/images/methods/japanese_drip.png', 'Japanese Drip', FALSE, 120, 210, 15),
    ('vietnam_drip', 'Vietnam Drip', 'Vietnam Drip (Phin)', 'STEEP_AND_RELEASE_IMMERSION', 'assets/images/methods/vietnam_drip.png', 'Vietnam Drip', FALSE, 180, 360, 16),
    ('origami', 'Origami Dripper', 'Origami Dripper', 'PERCOLATION_POUR_OVER', 'assets/images/methods/origami_dripper.png', 'Origami', FALSE, 105, 220, 17),
    ('turkish_coffee', 'Turkish Coffee', 'Kopi Turki (Cezve)', 'THERMAL_CUE_PHASE_BREWER', 'assets/images/methods/turkish_coffee.png', 'Turkish Coffee', FALSE, 110, 210, 18)
ON CONFLICT (slug) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    display_name_id = EXCLUDED.display_name_id,
    archetype = EXCLUDED.archetype,
    icon_asset_path = EXCLUDED.icon_asset_path,
    filter_key = EXCLUDED.filter_key,
    is_background_alarm_mode = EXCLUDED.is_background_alarm_mode,
    default_min_duration_sec = EXCLUDED.default_min_duration_sec,
    default_max_duration_sec = EXCLUDED.default_max_duration_sec,
    sort_order = EXCLUDED.sort_order;

CREATE TABLE IF NOT EXISTS long_brew_scheduled_alarms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipe_id INT,
    recipe_title VARCHAR(140) NOT NULL,
    method_slug VARCHAR(48) NOT NULL DEFAULT 'cold_brew',
    brew_mode VARCHAR(32) NOT NULL DEFAULT 'IMMERSION_FRIDGE',
    coffee_dose_g REAL NOT NULL DEFAULT 50.0,
    water_ml REAL NOT NULL DEFAULT 500.0,
    ambient_temp_c REAL NOT NULL DEFAULT 4.0,
    drip_rate_bpm SMALLINT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    target_finish_at TIMESTAMPTZ NOT NULL,
    total_duration_sec INT NOT NULL,
    checkpoints_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    alarm_status VARCHAR(24) NOT NULL DEFAULT 'ACTIVE',
    snooze_count SMALLINT NOT NULL DEFAULT 0,
    last_push_sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_long_brew_alarms_due
    ON long_brew_scheduled_alarms (alarm_status, target_finish_at);
CREATE INDEX IF NOT EXISTS idx_long_brew_alarms_user
    ON long_brew_scheduled_alarms (user_id, alarm_status, target_finish_at DESC);
