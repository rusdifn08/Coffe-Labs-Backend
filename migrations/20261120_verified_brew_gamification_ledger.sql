-- True Proof-of-Brew Verification & Universal Gamification Ledger
DO $$ BEGIN
    CREATE TYPE brew_session_verification_status AS ENUM (
        'ACTIVE',
        'VERIFIED_COMPLETED',
        'ABANDONED_EARLY',
        'REJECTED_INVALID_DURATION',
        'EXPIRED'
    );
EXCEPTION WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE gamification_xp_source AS ENUM (
        'VERIFIED_BREW',
        'FIRST_TIME_RECIPE_BONUS',
        'FIRST_TIME_METHOD_BONUS',
        'DAILY_STREAK_BONUS',
        'LMS_LESSON_COMPLETE',
        'EBOOK_CHAPTER_COMPLETE',
        'GAME_BREW_3D_COMPLETE',
        'AI_BEAN_SCAN',
        'SOCIAL_RECIPE_TRIED_BY_OTHERS',
        'ADMIN_BONUS'
    );
EXCEPTION WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS user_gamification_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    total_xp BIGINT NOT NULL DEFAULT 0 CHECK (total_xp >= 0),
    current_level INT NOT NULL DEFAULT 1 CHECK (current_level >= 1),
    barista_rank_title VARCHAR(60) NOT NULL DEFAULT 'Home Brewer',
    level_start_xp BIGINT NOT NULL DEFAULT 0,
    next_level_xp BIGINT NOT NULL DEFAULT 150,
    level_progress_ratio REAL NOT NULL DEFAULT 0.0,
    total_verified_brews_count INT NOT NULL DEFAULT 0,
    unique_methods_mastered_count SMALLINT NOT NULL DEFAULT 0,
    unique_recipes_tried_count INT NOT NULL DEFAULT 0,
    current_streak_days INT NOT NULL DEFAULT 0,
    longest_streak_days INT NOT NULL DEFAULT 0,
    last_verified_brew_date DATE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS brew_verification_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    method_id VARCHAR(50) NOT NULL,
    method_display_name VARCHAR(80) NOT NULL,
    archetype VARCHAR(30) NOT NULL DEFAULT 'POUR_OVER',
    recipe_id VARCHAR(80) NOT NULL,
    recipe_title VARCHAR(160) NOT NULL,
    bean_name VARCHAR(120),
    roastery_name VARCHAR(120),
    target_duration_sec INT NOT NULL CHECK (target_duration_sec > 0),
    total_steps_count SMALLINT NOT NULL DEFAULT 1,
    server_started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_checkpoint_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    server_completed_at TIMESTAMPTZ,
    reported_active_elapsed_ms BIGINT NOT NULL DEFAULT 0,
    reported_paused_ms BIGINT NOT NULL DEFAULT 0,
    completed_steps_mask BIGINT NOT NULL DEFAULT 0,
    pour_accuracy_pct REAL,
    status brew_session_verification_status NOT NULL DEFAULT 'ACTIVE',
    rejection_reason VARCHAR(160),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_brew_verify_sessions_user_active
    ON brew_verification_sessions (user_id, status, server_started_at DESC);

CREATE TABLE IF NOT EXISTS verified_brew_completions (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL UNIQUE REFERENCES brew_verification_sessions(id) ON DELETE RESTRICT,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    method_id VARCHAR(50) NOT NULL,
    method_display_name VARCHAR(80) NOT NULL,
    recipe_id VARCHAR(80) NOT NULL,
    recipe_title VARCHAR(160) NOT NULL,
    bean_name VARCHAR(120),
    roastery_name VARCHAR(120),
    coffee_dose_g REAL NOT NULL DEFAULT 15.0,
    water_total_ml REAL NOT NULL DEFAULT 225.0,
    water_temp_c SMALLINT NOT NULL DEFAULT 91,
    actual_duration_sec INT NOT NULL,
    pour_accuracy_pct REAL,
    xp_awarded INT NOT NULL DEFAULT 0,
    user_rating SMALLINT CHECK (user_rating BETWEEN 1 AND 5),
    tasting_notes_comment TEXT,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_verified_brews_user_recent
    ON verified_brew_completions (user_id, completed_at DESC);

CREATE TABLE IF NOT EXISTS user_method_brew_stats (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    method_id VARCHAR(50) NOT NULL,
    completed_count INT NOT NULL DEFAULT 0 CHECK (completed_count >= 0),
    total_brew_seconds BIGINT NOT NULL DEFAULT 0,
    first_brewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_brewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, method_id)
);

CREATE TABLE IF NOT EXISTS user_tried_recipes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipe_id VARCHAR(80) NOT NULL,
    recipe_title VARCHAR(160) NOT NULL,
    method_id VARCHAR(50) NOT NULL,
    times_completed INT NOT NULL DEFAULT 1 CHECK (times_completed >= 1),
    best_accuracy_pct REAL,
    last_duration_sec INT NOT NULL,
    first_completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, recipe_id)
);

CREATE INDEX IF NOT EXISTS idx_user_tried_recipes_recent
    ON user_tried_recipes (user_id, last_completed_at DESC);

CREATE TABLE IF NOT EXISTS gamification_xp_ledger (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    idempotency_key VARCHAR(160) NOT NULL UNIQUE,
    source_kind gamification_xp_source NOT NULL,
    xp_delta INT NOT NULL CHECK (xp_delta > 0),
    xp_before BIGINT NOT NULL,
    xp_after BIGINT NOT NULL,
    level_before INT NOT NULL,
    level_after INT NOT NULL,
    reference_entity_id VARCHAR(100),
    label_id VARCHAR(140) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_xp_ledger_user_history
    ON gamification_xp_ledger (user_id, id DESC);
