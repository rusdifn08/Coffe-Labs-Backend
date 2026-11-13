-- ============================================================================
-- Coffee Labs Social Coffee (Modul 3.2): Public Profile CQRS,
-- Barista Rank Gamification & Interactive Activity Thread Feed
-- ============================================================================

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'thread_kind') THEN
        CREATE TYPE thread_kind AS ENUM (
            'STANDARD_POST',
            'BREW_LOG',
            'CREATED_RECIPE',
            'REPLY',
            'REPOST_QUOTE'
        );
    END IF;
END
$$;

-- 1. Tabel Proyeksi CQRS Profil Publik & Barista Rank (Read-Optimized O(1))
CREATE TABLE IF NOT EXISTS user_profile_projections (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    username VARCHAR(40) NOT NULL UNIQUE,
    display_name VARCHAR(80) NOT NULL,
    bio TEXT NOT NULL DEFAULT '',
    avatar_url TEXT,
    banner_url TEXT,
    account_tier VARCHAR(20) NOT NULL DEFAULT 'Trial',

    -- Gamification & Barista Rank Metrics
    total_xp BIGINT NOT NULL DEFAULT 0,
    barista_level INT NOT NULL DEFAULT 1,
    barista_rank_title VARCHAR(50) NOT NULL DEFAULT 'Home Brewer',
    level_progress_ratio REAL NOT NULL DEFAULT 0.0,
    current_streak_days INT NOT NULL DEFAULT 0,
    unlocked_badges_bitmap BYTEA,
    top_featured_badges JSONB NOT NULL DEFAULT '[]'::jsonb,

    -- Denormalized Social Counters (Diperbarui secara atomik)
    followers_count INT NOT NULL DEFAULT 0,
    following_count INT NOT NULL DEFAULT 0,
    total_threads_count INT NOT NULL DEFAULT 0,
    total_brews_count INT NOT NULL DEFAULT 0,
    total_recipes_created_count INT NOT NULL DEFAULT 0,
    total_recipe_tries_received INT NOT NULL DEFAULT 0,

    -- Live / Recent Brew Telemetry Snapshot
    recent_brew_snapshot JSONB,

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_profile_proj_username
    ON user_profile_projections (username);
CREATE INDEX IF NOT EXISTS idx_profile_proj_xp_rank
    ON user_profile_projections (total_xp DESC, user_id);

-- 2. Tabel Relasi Sosial Follow Graph
CREATE TABLE IF NOT EXISTS social_follows (
    follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (follower_id, following_id),
    CONSTRAINT chk_no_self_follow CHECK (follower_id <> following_id)
);

CREATE INDEX IF NOT EXISTS idx_social_follows_following
    ON social_follows (following_id, created_at DESC);

-- 3. Tabel Utama Interactive Coffee Threads & Activity Feed (Keyset UUIDv7)
CREATE TABLE IF NOT EXISTS social_threads (
    id UUID PRIMARY KEY,
    author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind thread_kind NOT NULL DEFAULT 'STANDARD_POST',

    -- Hierarki Komentar Bersarang (Nested Replies) & Quote
    parent_thread_id UUID REFERENCES social_threads(id) ON DELETE CASCADE,
    root_thread_id UUID REFERENCES social_threads(id) ON DELETE CASCADE,
    quoted_thread_id UUID REFERENCES social_threads(id) ON DELETE SET NULL,
    depth_level SMALLINT NOT NULL DEFAULT 0,

    -- Konten & Rich Embedded Coffee Attachments
    body_text TEXT NOT NULL DEFAULT '',
    has_media BOOLEAN NOT NULL DEFAULT FALSE,
    media_urls JSONB NOT NULL DEFAULT '[]'::jsonb,
    attachment_type VARCHAR(30),
    attachment_payload JSONB,

    -- Denormalized Engagement Counters
    likes_count INT NOT NULL DEFAULT 0,
    replies_count INT NOT NULL DEFAULT 0,
    reposts_count INT NOT NULL DEFAULT 0,
    try_recipe_count INT NOT NULL DEFAULT 0,

    is_pinned_on_profile BOOLEAN NOT NULL DEFAULT FALSE,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5 Composite B-Tree Partial Indexes (< 0.5 ms Keyset Pagination)
CREATE INDEX IF NOT EXISTS idx_threads_profile_all
    ON social_threads (author_id, is_pinned_on_profile DESC, id DESC)
    WHERE is_deleted = FALSE AND depth_level = 0;

CREATE INDEX IF NOT EXISTS idx_threads_profile_by_kind
    ON social_threads (author_id, kind, id DESC)
    WHERE is_deleted = FALSE;

CREATE INDEX IF NOT EXISTS idx_threads_profile_media
    ON social_threads (author_id, id DESC)
    WHERE is_deleted = FALSE AND has_media = TRUE;

CREATE INDEX IF NOT EXISTS idx_threads_nested_replies
    ON social_threads (root_thread_id, id ASC)
    WHERE is_deleted = FALSE AND root_thread_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_threads_global_feed
    ON social_threads (id DESC)
    WHERE is_deleted = FALSE AND depth_level = 0;

-- 4. Tabel Interaksi Likes & Reposts (Idempotent)
CREATE TABLE IF NOT EXISTS social_thread_likes (
    thread_id UUID NOT NULL REFERENCES social_threads(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (thread_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_thread_likes_user_lookup
    ON social_thread_likes (user_id, thread_id);

CREATE TABLE IF NOT EXISTS social_thread_reposts (
    thread_id UUID NOT NULL REFERENCES social_threads(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (thread_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_thread_reposts_user_lookup
    ON social_thread_reposts (user_id, thread_id);
