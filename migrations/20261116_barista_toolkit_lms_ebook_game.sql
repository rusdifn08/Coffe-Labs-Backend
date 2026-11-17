-- =============================================================================
-- COFFEE LABS MODUL 2.1: BARISTA LEARNING BREW (LMS, E-BOOK DRM, GAME BREW V60)
-- Migration: 20261116_barista_toolkit_lms_ebook_game.sql
-- =============================================================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'course_difficulty_level') THEN
        CREATE TYPE course_difficulty_level AS ENUM ('NOVICE', 'ARTISAN', 'ALCHEMIST', 'WBRC_MASTER');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'lms_lesson_kind') THEN
        CREATE TYPE lms_lesson_kind AS ENUM ('HLS_VIDEO', 'INTERACTIVE_SIMULATION', 'QUIZ_ASSESSMENT');
    END IF;
END
$$;

-- 1. Tabel Katalog Kursus LMS (Structured Learning Paths)
CREATE TABLE IF NOT EXISTS lms_courses (
    id UUID PRIMARY KEY,
    slug VARCHAR(80) NOT NULL UNIQUE,
    title VARCHAR(140) NOT NULL,
    subtitle VARCHAR(240) NOT NULL,
    instructor_name VARCHAR(100) NOT NULL,
    difficulty course_difficulty_level NOT NULL DEFAULT 'NOVICE',
    required_tier VARCHAR(20) NOT NULL DEFAULT 'Trial',
    thumbnail_url TEXT NOT NULL,
    total_modules_count SMALLINT NOT NULL DEFAULT 0,
    total_duration_sec INT NOT NULL DEFAULT 0,
    xp_reward_on_complete INT NOT NULL DEFAULT 250,
    badge_slug_reward VARCHAR(60),
    is_published BOOLEAN NOT NULL DEFAULT TRUE,
    display_order SMALLINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Tabel Modul & Pelajaran Video HLS / Kuis Interaktif
CREATE TABLE IF NOT EXISTS lms_lessons (
    id UUID PRIMARY KEY,
    course_id UUID NOT NULL REFERENCES lms_courses(id) ON DELETE CASCADE,
    module_number SMALLINT NOT NULL,
    lesson_number SMALLINT NOT NULL,
    title VARCHAR(140) NOT NULL,
    description TEXT NOT NULL,
    kind lms_lesson_kind NOT NULL DEFAULT 'HLS_VIDEO',
    hls_manifest_path TEXT,
    duration_sec INT NOT NULL DEFAULT 0,
    quiz_questions_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    key_takeaways_markdown TEXT,
    xp_reward INT NOT NULL DEFAULT 50,
    CONSTRAINT uq_course_module_lesson UNIQUE (course_id, module_number, lesson_number)
);

-- 3. Pelacakan Progres Belajar LMS per Pengguna (100-Bit Segment Bitmask Anti-Skip)
CREATE TABLE IF NOT EXISTS user_lms_lesson_progress (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lesson_id UUID NOT NULL REFERENCES lms_lessons(id) ON DELETE CASCADE,
    course_id UUID NOT NULL REFERENCES lms_courses(id) ON DELETE CASCADE,
    watched_segments_bitmap BYTEA NOT NULL DEFAULT '\x00000000000000000000000000'::bytea,
    last_position_sec INT NOT NULL DEFAULT 0,
    completion_pct SMALLINT NOT NULL DEFAULT 0,
    quiz_score_pct SMALLINT,
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, lesson_id)
);

CREATE INDEX IF NOT EXISTS idx_lms_progress_user_course
    ON user_lms_lesson_progress (user_id, course_id);

-- 4. Katalog E-Book Terproteksi (AES-256-GCM Envelope DRM)
CREATE TABLE IF NOT EXISTS toolkit_ebooks (
    id UUID PRIMARY KEY,
    book_code VARCHAR(30) NOT NULL UNIQUE,
    title VARCHAR(160) NOT NULL,
    author VARCHAR(120) NOT NULL,
    synopsis TEXT NOT NULL,
    cover_image_url TEXT NOT NULL,
    category VARCHAR(50) NOT NULL,
    required_tier VARCHAR(20) NOT NULL DEFAULT 'Trial',
    total_pages INT NOT NULL,
    encrypted_asset_url TEXT NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    sha256_ciphertext_hash VARCHAR(64) NOT NULL,
    wrapped_master_cek BYTEA NOT NULL,
    toc_chapters_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    xp_reward_on_finish INT NOT NULL DEFAULT 300,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Progres Baca, Bookmark & Highlight E-Book per Pengguna
CREATE TABLE IF NOT EXISTS user_ebook_progress (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ebook_id UUID NOT NULL REFERENCES toolkit_ebooks(id) ON DELETE CASCADE,
    current_page INT NOT NULL DEFAULT 1,
    highest_page_reached INT NOT NULL DEFAULT 1,
    read_pages_bitmap BYTEA,
    completion_pct SMALLINT NOT NULL DEFAULT 0,
    bookmarks_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    highlights_notes_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    last_read_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, ebook_id)
);

-- 6. Alur Cerita & Progres "Game Brew: Kisah Seduh V60"
CREATE TABLE IF NOT EXISTS game_brew_chapters (
    chapter_number SMALLINT PRIMARY KEY,
    slug VARCHAR(60) NOT NULL UNIQUE,
    title VARCHAR(120) NOT NULL,
    story_location VARCHAR(100) NOT NULL,
    npc_mentor_name VARCHAR(80) NOT NULL,
    target_recipe_slug VARCHAR(80) NOT NULL,
    physics_config_json JSONB NOT NULL,
    story_dialogue_json JSONB NOT NULL,
    xp_reward_3_stars INT NOT NULL DEFAULT 200
);

CREATE TABLE IF NOT EXISTS user_game_brew_progress (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chapter_number SMALLINT NOT NULL REFERENCES game_brew_chapters(chapter_number) ON DELETE CASCADE,
    stars_earned SMALLINT NOT NULL DEFAULT 0 CHECK (stars_earned BETWEEN 0 AND 3),
    high_score_accuracy_pct REAL NOT NULL DEFAULT 0.0,
    best_extraction_yield_pct REAL,
    attempts_count INT NOT NULL DEFAULT 1,
    is_unlocked BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, chapter_number)
);
