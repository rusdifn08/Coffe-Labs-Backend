-- =============================================================================
-- COFFEE LABS ENTERPRISE GAMIFICATION & ANTI-CHEAT ENGINE (NEONDB POSTGRESQL)
-- Migration: 20261102_gamification_system.sql
-- =============================================================================

-- 1. GAMIFICATION PROFILES (Mastery XP, Level, Streak & RoaringBitmap Badges)
CREATE TABLE IF NOT EXISTS gamification_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    total_xp BIGINT NOT NULL DEFAULT 0,
    current_level INT NOT NULL DEFAULT 1,
    current_streak INT NOT NULL DEFAULT 0,
    longest_streak INT NOT NULL DEFAULT 0,
    streak_freezes_available INT NOT NULL DEFAULT 0,
    last_brew_local_date DATE,
    timezone VARCHAR(64) NOT NULL DEFAULT 'Asia/Jakarta',
    unlocked_badges_bitmap BYTEA NOT NULL DEFAULT '\x'::bytea,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_gamification_leaderboard_xp
    ON gamification_profiles (total_xp DESC, current_level DESC, updated_at ASC);

CREATE INDEX IF NOT EXISTS idx_gamification_streak_date
    ON gamification_profiles (last_brew_local_date, current_streak DESC);

-- 2. XP LEDGER TRANSACTIONS (Immutable Append-Only Double-Entry Audit Log)
CREATE TABLE IF NOT EXISTS xp_ledger_transactions (
    id VARCHAR(36) PRIMARY KEY, -- ULID sortable unique identifier
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action_type VARCHAR(32) NOT NULL CHECK (
        action_type IN (
            'BREW_COMPLETED',
            'MASTERCLASS_FINISHED',
            'RECIPE_SHARED',
            'STREAK_BONUS',
            'QUEST_REWARD'
        )
    ),
    base_xp INT NOT NULL CHECK (base_xp >= 0),
    tier_multiplier NUMERIC(3,2) NOT NULL DEFAULT 1.00,
    final_xp INT NOT NULL CHECK (final_xp >= 0),
    reference_id UUID,
    idempotency_key VARCHAR(128) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_xp_ledger_user_created
    ON xp_ledger_transactions (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_xp_ledger_weekly_leaderboard
    ON xp_ledger_transactions (created_at DESC, user_id, final_xp);

-- 3. BREW VERIFICATION SESSIONS (Anti-Cheat Duration & Telemetry Gatekeeper)
CREATE TABLE IF NOT EXISTS brew_verification_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipe_id UUID NOT NULL,
    target_duration_sec INT NOT NULL CHECK (target_duration_sec >= 15),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    is_valid BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR(32) NOT NULL DEFAULT 'STARTED'
);

CREATE INDEX IF NOT EXISTS idx_brew_sessions_user_status
    ON brew_verification_sessions (user_id, status, started_at DESC);

-- 4. ACHIEVEMENTS CATALOG & USER PROGRESS (20 Specialty Coffee Badges)
CREATE TABLE IF NOT EXISTS achievements_catalog (
    id INT PRIMARY KEY, -- Bit index in RoaringBitmap (1..20)
    slug VARCHAR(64) UNIQUE NOT NULL,
    title VARCHAR(120) NOT NULL,
    category VARCHAR(40) NOT NULL, -- PRECISION_ALCHEMIST | ORIGIN_EXPLORER | TECHNIQUE_GRANDMASTER | COMMUNITY_ROASTER
    description TEXT NOT NULL,
    criteria_json JSONB NOT NULL,
    xp_reward INT NOT NULL DEFAULT 100,
    rive_artboard VARCHAR(80) NOT NULL,
    rarity VARCHAR(24) NOT NULL DEFAULT 'RARE', -- COMMON | RARE | EPIC | LEGENDARY | MYTHIC
    tier_required VARCHAR(20) NOT NULL DEFAULT 'trial',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS user_achievement_progress (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    achievement_id INT NOT NULL REFERENCES achievements_catalog(id) ON DELETE CASCADE,
    current_value INT NOT NULL DEFAULT 0,
    target_value INT NOT NULL DEFAULT 1,
    is_unlocked BOOLEAN NOT NULL DEFAULT FALSE,
    unlocked_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, achievement_id)
);

CREATE INDEX IF NOT EXISTS idx_user_achievement_unlocked
    ON user_achievement_progress (user_id, is_unlocked, updated_at DESC);

-- 5. SEED 20 SPECIALTY COFFEE BADGES (COMPLETE JSONB CRITERIA)
INSERT INTO achievements_catalog (
    id, slug, title, category, description, criteria_json, xp_reward, rive_artboard, rarity, tier_required
) VALUES
-- Category 1: Precision Alchemist (Ketepatan Waktu & Rasio Seduh V60)
(1, 'first_golden_bloom', 'First Golden Bloom', 'PRECISION_ALCHEMIST',
 'Menyelesaikan fase blooming 30-45 detik dengan rasio air 1:3 secara presisi.',
 '{"metric":"bloom_precision_count","target":1,"tolerance_pct":10,"ratio":"1:3"}'::jsonb,
 100, 'Badge_GoldenBloom', 'COMMON', 'trial'),
(2, 'metronome_pour_v60', 'Metronome Pour V60', 'PRECISION_ALCHEMIST',
 'Menyelesaikan 5 seduhan Hario V60 dengan deviasi durasi total kurang dari 3 detik dari target resep.',
 '{"metric":"exact_duration_brews","method":"V60","target":5,"max_deviation_sec":3}'::jsonb,
 250, 'Badge_MetronomeV60', 'RARE', 'trial'),
(3, 'flow_rate_maestro', 'Flow Rate Maestro', 'PRECISION_ALCHEMIST',
 'Menjaga laju tuangan stabil 5.0 - 6.5 ml/detik sepanjang 10 sesi seduh berturut-turut.',
 '{"metric":"steady_flow_sessions","target":10,"min_flow_ml_s":5.0,"max_flow_ml_s":6.5}'::jsonb,
 400, 'Badge_FlowMaestro', 'EPIC', 'pro'),
(4, 'thermal_equilibrium', 'Thermal Equilibrium', 'PRECISION_ALCHEMIST',
 'Menyeduh 15 resep light-roast menggunakan profil suhu presisi 91°C - 93°C.',
 '{"metric":"temp_zone_brews","target":15,"min_temp_c":91,"max_temp_c":93}'::jsonb,
 350, 'Badge_ThermalEquilibrium', 'RARE', 'trial'),
(5, 'extraction_alchemist_20', '20% Extraction Alchemist', 'PRECISION_ALCHEMIST',
 'Mencapai zona Golden Cup (Skor Sensorik >= 4.75) pada 20 sesi seduh terverifikasi.',
 '{"metric":"golden_cup_sessions","target":20,"min_sensory_score":4.75}'::jsonb,
 750, 'Badge_ExtractionAlchemist', 'LEGENDARY', 'pro'),

-- Category 2: Origin Explorer (Eksplorasi Biji Kopi Nusantara & Dunia)
(6, 'kintamani_citrus_seeker', 'Kintamani Citrus Seeker', 'ORIGIN_EXPLORER',
 'Menyeduh biji kopi single-origin Bali Kintamani (Washed atau Anaerobic Natural).',
 '{"metric":"origin_brewed","origin":"Bali Kintamani","target":1}'::jsonb,
 150, 'Badge_KintamaniExplorer', 'COMMON', 'trial'),
(7, 'gayo_highland_pioneer', 'Gayo Highland Pioneer', 'ORIGIN_EXPLORER',
 'Menyeduh 3 varietas Aceh Gayo (Bourbon, Timtim, atau Abyssi) dengan proses berbeda.',
 '{"metric":"origin_brewed","origin":"Aceh Gayo","target":3}'::jsonb,
 250, 'Badge_GayoPioneer', 'RARE', 'trial'),
(8, 'flores_toraja_voyager', 'Archipelago Voyager', 'ORIGIN_EXPLORER',
 'Menjelajahi 5 origin Nusantara: Bali, Gayo, Flores Bajawa, Toraja, dan Gunung Halu.',
 '{"metric":"distinct_indonesian_origins","origins":["Bali","Gayo","Flores","Toraja","Halu"],"target":5}'::jsonb,
 500, 'Badge_ArchipelagoVoyager', 'EPIC', 'trial'),
(9, 'carbonic_maceration_connoisseur', 'Anaerobic & CM Connoisseur', 'ORIGIN_EXPLORER',
 'Menyeduh 5 biji kopi eksperimental berproses Anaerobic Natural atau Carbonic Maceration.',
 '{"metric":"process_brewed","processes":["Anaerobic Natural","Carbonic Maceration","Wine"],"target":5}'::jsonb,
 450, 'Badge_CarbonicConnoisseur', 'EPIC', 'pro'),
(10, 'panama_geisha_sovereign', 'Geisha Varietal Sovereign', 'ORIGIN_EXPLORER',
 'Menyelesaikan seduhan biji varietas langka Geisha / Gesha dengan akurasi waktu >= 95%.',
 '{"metric":"variety_brewed","variety":"Geisha","target":1,"min_accuracy_pct":95}'::jsonb,
 900, 'Badge_GeishaSovereign', 'MYTHIC', 'pro'),

-- Category 3: Technique Grandmaster (Masterclass & Teknik Juara Dunia)
(11, 'tetsu_46_disciple', 'Tetsu Kasuya 4:6 Disciple', 'TECHNIQUE_GRANDMASTER',
 'Menyelesaikan 5 seduhan menggunakan metode 4:6 (5 tahapan tuangan keseimbangan manis-asam).',
 '{"metric":"technique_completed","technique":"TETSU_4_6","target":5}'::jsonb,
 300, 'Badge_Tetsu46', 'RARE', 'trial'),
(12, 'wbrc_sandwich_specialist', 'WBrC Sandwich Specialist', 'TECHNIQUE_GRANDMASTER',
 'Menyelesaikan teknik juara dunia WBrC (Kombinasi Immersion Switch + Percolation V60).',
 '{"metric":"technique_completed","technique":"WBRC_SANDWICH","target":3}'::jsonb,
 550, 'Badge_WbrcSandwich', 'EPIC', 'pro'),
(13, 'masterclass_scholar', 'Masterclass Scholar', 'TECHNIQUE_GRANDMASTER',
 'Menyelesaikan seluruh modul Pre-Brew Masterclass (Kimia Air, Distribusi Gilingan, dan Turbulensi).',
 '{"metric":"masterclass_modules_completed","target":5,"pro_exclusive":true}'::jsonb,
 600, 'Badge_MasterclassScholar', 'EPIC', 'pro'),
(14, 'osmotic_flow_zen', 'Osmotic Flow Zen Master', 'TECHNIQUE_GRANDMASTER',
 'Menyeduh 5 sesi Dark/Medium-Dark Roast menggunakan teknik Osmotic Flow tengah tanpa agitasi dinding.',
 '{"metric":"technique_completed","technique":"OSMOTIC_FLOW","target":5}'::jsonb,
 400, 'Badge_OsmoticZen', 'RARE', 'trial'),
(15, 'sensory_lexicon_calibrator', 'Q-Grader Sensory Calibrator', 'TECHNIQUE_GRANDMASTER',
 'Mencatat 15 jurnal Cupping lengkap dengan evaluasi Acidity, Sweetness, Body, dan Aftertaste.',
 '{"metric":"cupping_logs_completed","target":15}'::jsonb,
 700, 'Badge_SensoryCalibrator', 'LEGENDARY', 'pro'),

-- Category 4: Community Roaster (Sosial Media, Re-Brew & Streak)
(16, 'open_source_brewer', 'Open-Source Barista', 'COMMUNITY_ROASTER',
 'Membagikan resep V60 pertama ke Community Feed Coffee Labs.',
 '{"metric":"recipes_shared","target":1}'::jsonb,
 150, 'Badge_OpenSourceBrewer', 'COMMON', 'trial'),
(17, 'viral_recipe_architect', 'Viral Recipe Architect', 'COMMUNITY_ROASTER',
 'Resep yang Anda bagikan berhasil di-copy dan diseduh ulang (re-brewed) oleh 10 barista lain.',
 '{"metric":"recipe_rebrews_received","target":10}'::jsonb,
 650, 'Badge_ViralArchitect', 'EPIC', 'trial'),
(18, 'streak_flame_7', '7-Day Morning Ritual', 'COMMUNITY_ROASTER',
 'Menjaga Daily Brewing Streak selama 7 hari berturut-turut tanpa putus.',
 '{"metric":"streak_days","target":7}'::jsonb,
 300, 'Badge_Streak7', 'RARE', 'trial'),
(19, 'streak_immortal_30', '30-Day Caffeine Immortal', 'COMMUNITY_ROASTER',
 'Menjaga Daily Brewing Streak selama 30 hari penuh.',
 '{"metric":"streak_days","target":30}'::jsonb,
 1200, 'Badge_Streak30', 'LEGENDARY', 'pro'),
(20, 'head_roaster_guild_leader', 'Guild Head Roaster', 'COMMUNITY_ROASTER',
 'Membuat Custom Brewing Challenge untuk komunitas atau meraih Top 3 Leaderboard Mingguan.',
 '{"metric":"business_challenge_or_top3","target":1}'::jsonb,
 1500, 'Badge_GuildHeadRoaster', 'MYTHIC', 'business')
ON CONFLICT (slug) DO UPDATE SET
    title = EXCLUDED.title,
    description = EXCLUDED.description,
    criteria_json = EXCLUDED.criteria_json,
    xp_reward = EXCLUDED.xp_reward,
    rive_artboard = EXCLUDED.rive_artboard,
    rarity = EXCLUDED.rarity,
    tier_required = EXCLUDED.tier_required;
