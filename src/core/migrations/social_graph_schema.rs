pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'follow_edge_status') THEN
                CREATE TYPE follow_edge_status AS ENUM ('ACCEPTED', 'PENDING');
            END IF;
        END
        $$"#,
        r#"ALTER TABLE user_profile_projections
            ADD COLUMN IF NOT EXISTS is_private_account BOOLEAN NOT NULL DEFAULT FALSE,
            ADD COLUMN IF NOT EXISTS pending_follow_requests_count INT NOT NULL DEFAULT 0"#,
        r#"CREATE TABLE IF NOT EXISTS user_follows (
            follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            status follow_edge_status NOT NULL DEFAULT 'ACCEPTED',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            accepted_at TIMESTAMPTZ,
            PRIMARY KEY (follower_id, following_id),
            CONSTRAINT chk_user_follows_no_self CHECK (follower_id <> following_id)
        )"#,
        r#"ALTER TABLE user_follows
            ADD COLUMN IF NOT EXISTS status follow_edge_status NOT NULL DEFAULT 'ACCEPTED',
            ADD COLUMN IF NOT EXISTS accepted_at TIMESTAMPTZ DEFAULT NOW()"#,
        r#"DO $$
        BEGIN
            IF EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public' AND table_name = 'social_follows' AND column_name = 'follower_id'
            ) THEN
                INSERT INTO user_follows (follower_id, following_id, status, created_at, accepted_at)
                SELECT follower_id, following_id, 'ACCEPTED'::follow_edge_status, created_at, created_at
                FROM social_follows
                ON CONFLICT (follower_id, following_id) DO NOTHING;
            ELSIF EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public' AND table_name = 'social_follows' AND column_name = 'follower_username'
            ) THEN
                INSERT INTO user_follows (follower_id, following_id, status, created_at, accepted_at)
                SELECT f.id, t.id, 'ACCEPTED'::follow_edge_status, sf.created_at, sf.created_at
                FROM social_follows sf
                INNER JOIN users f ON LOWER(f.username) = LOWER(sf.follower_username)
                INNER JOIN users t ON LOWER(t.username) = LOWER(sf.following_username)
                WHERE f.id <> t.id
                ON CONFLICT (follower_id, following_id) DO NOTHING;
            END IF;
        END
        $$"#,
        "CREATE INDEX IF NOT EXISTS idx_user_follows_reverse_lookup ON user_follows (following_id, follower_id, status)",
        "CREATE INDEX IF NOT EXISTS idx_user_follows_followers_list ON user_follows (following_id, created_at DESC, follower_id) WHERE status = 'ACCEPTED'",
        "CREATE INDEX IF NOT EXISTS idx_user_follows_following_list ON user_follows (follower_id, created_at DESC, following_id) WHERE status = 'ACCEPTED'",
        "CREATE INDEX IF NOT EXISTS idx_user_follows_pending_requests ON user_follows (following_id, created_at DESC, follower_id) WHERE status = 'PENDING'",
        // Social Coffee v2 Upgrades: Edit, Delete, Repost, Save/Bookmark, and Moderation Reports
        r#"ALTER TABLE social_posts
            ADD COLUMN IF NOT EXISTS reposts_count INT NOT NULL DEFAULT 0,
            ADD COLUMN IF NOT EXISTS is_edited BOOLEAN NOT NULL DEFAULT FALSE"#,
        r#"CREATE TABLE IF NOT EXISTS social_post_reposts (
            post_id UUID NOT NULL,
            username VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (post_id, username)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS social_post_saves (
            post_id UUID NOT NULL,
            username VARCHAR(100) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (post_id, username)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS social_post_reports (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            post_id UUID NOT NULL,
            reporter_username VARCHAR(100) NOT NULL,
            reason VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_social_post_saves_username ON social_post_saves (username, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_social_post_reposts_username ON social_post_reposts (username, created_at DESC)",
    ]
}
