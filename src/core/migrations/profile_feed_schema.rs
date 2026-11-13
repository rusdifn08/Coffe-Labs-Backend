pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
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
        $$"#,
        r#"CREATE TABLE IF NOT EXISTS user_profile_projections (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            username VARCHAR(40) NOT NULL UNIQUE,
            display_name VARCHAR(80) NOT NULL,
            bio TEXT NOT NULL DEFAULT '',
            avatar_url TEXT,
            banner_url TEXT,
            account_tier VARCHAR(20) NOT NULL DEFAULT 'Trial',
            total_xp BIGINT NOT NULL DEFAULT 0,
            barista_level INT NOT NULL DEFAULT 1,
            barista_rank_title VARCHAR(50) NOT NULL DEFAULT 'Home Brewer',
            level_progress_ratio REAL NOT NULL DEFAULT 0.0,
            current_streak_days INT NOT NULL DEFAULT 0,
            unlocked_badges_bitmap BYTEA,
            top_featured_badges JSONB NOT NULL DEFAULT '[]'::jsonb,
            followers_count INT NOT NULL DEFAULT 0,
            following_count INT NOT NULL DEFAULT 0,
            total_threads_count INT NOT NULL DEFAULT 0,
            total_brews_count INT NOT NULL DEFAULT 0,
            total_recipes_created_count INT NOT NULL DEFAULT 0,
            total_recipe_tries_received INT NOT NULL DEFAULT 0,
            recent_brew_snapshot JSONB,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_profile_proj_username ON user_profile_projections (username)",
        "CREATE INDEX IF NOT EXISTS idx_profile_proj_xp_rank ON user_profile_projections (total_xp DESC, user_id)",
        r#"CREATE TABLE IF NOT EXISTS social_follows (
            follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (follower_id, following_id),
            CONSTRAINT chk_no_self_follow CHECK (follower_id <> following_id)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_social_follows_following ON social_follows (following_id, created_at DESC)",
        r#"CREATE TABLE IF NOT EXISTS social_threads (
            id UUID PRIMARY KEY,
            author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            kind thread_kind NOT NULL DEFAULT 'STANDARD_POST',
            parent_thread_id UUID REFERENCES social_threads(id) ON DELETE CASCADE,
            root_thread_id UUID REFERENCES social_threads(id) ON DELETE CASCADE,
            quoted_thread_id UUID REFERENCES social_threads(id) ON DELETE SET NULL,
            depth_level SMALLINT NOT NULL DEFAULT 0,
            body_text TEXT NOT NULL DEFAULT '',
            has_media BOOLEAN NOT NULL DEFAULT FALSE,
            media_urls JSONB NOT NULL DEFAULT '[]'::jsonb,
            attachment_type VARCHAR(30),
            attachment_payload JSONB,
            likes_count INT NOT NULL DEFAULT 0,
            replies_count INT NOT NULL DEFAULT 0,
            reposts_count INT NOT NULL DEFAULT 0,
            try_recipe_count INT NOT NULL DEFAULT 0,
            is_pinned_on_profile BOOLEAN NOT NULL DEFAULT FALSE,
            is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_threads_profile_all ON social_threads (author_id, is_pinned_on_profile DESC, id DESC) WHERE is_deleted = FALSE AND depth_level = 0",
        "CREATE INDEX IF NOT EXISTS idx_threads_profile_by_kind ON social_threads (author_id, kind, id DESC) WHERE is_deleted = FALSE",
        "CREATE INDEX IF NOT EXISTS idx_threads_profile_media ON social_threads (author_id, id DESC) WHERE is_deleted = FALSE AND has_media = TRUE",
        "CREATE INDEX IF NOT EXISTS idx_threads_nested_replies ON social_threads (root_thread_id, id ASC) WHERE is_deleted = FALSE AND root_thread_id IS NOT NULL",
        "CREATE INDEX IF NOT EXISTS idx_threads_global_feed ON social_threads (id DESC) WHERE is_deleted = FALSE AND depth_level = 0",
        r#"CREATE TABLE IF NOT EXISTS social_thread_likes (
            thread_id UUID NOT NULL REFERENCES social_threads(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (thread_id, user_id)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_thread_likes_user_lookup ON social_thread_likes (user_id, thread_id)",
        r#"CREATE TABLE IF NOT EXISTS social_thread_reposts (
            thread_id UUID NOT NULL REFERENCES social_threads(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (thread_id, user_id)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_thread_reposts_user_lookup ON social_thread_reposts (user_id, thread_id)",
        r#"INSERT INTO user_profile_projections (
            user_id, username, display_name, bio, avatar_url, account_tier,
            total_xp, barista_level, barista_rank_title, current_streak_days, unlocked_badges_bitmap
        )
        SELECT
            u.id,
            LEFT(u.username, 40),
            LEFT(COALESCE(NULLIF(u.full_name, ''), u.username), 80),
            COALESCE(u.bio, 'Barista & V60 Enthusiast di Coffee Labs'),
            u.avatar_url,
            INITCAP(COALESCE(u.tier, 'trial')),
            COALESCE(g.total_xp, 0),
            COALESCE(g.current_level, 1),
            CASE
                WHEN COALESCE(g.current_level, 1) >= 80 THEN 'WBrC Grandmaster'
                WHEN COALESCE(g.current_level, 1) >= 50 THEN 'Head Roaster & Master'
                WHEN COALESCE(g.current_level, 1) >= 25 THEN 'Sensory Alchemist'
                WHEN COALESCE(g.current_level, 1) >= 10 THEN 'Pour-Over Artisan'
                ELSE 'Home Brewer'
            END,
            COALESCE(g.current_streak, 0),
            g.unlocked_badges_bitmap
        FROM users u
        LEFT JOIN gamification_profiles g ON g.user_id = u.id
        ON CONFLICT (user_id) DO NOTHING"#,
    ]
}
