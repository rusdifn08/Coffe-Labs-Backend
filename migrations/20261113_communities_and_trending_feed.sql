-- Module 3.5: Coffee Communities ("Coffee Clubs"), Secret ID Invite Engine, & For-You / Trending Feed
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'community_privacy_kind') THEN
        CREATE TYPE community_privacy_kind AS ENUM ('PUBLIC', 'RESTRICTED', 'SECRET_VAULT');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'community_member_role') THEN
        CREATE TYPE community_member_role AS ENUM ('OWNER', 'ADMIN', 'MODERATOR', 'BARISTA_MEMBER');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'community_channel_kind') THEN
        CREATE TYPE community_channel_kind AS ENUM ('REALTIME_CHAT', 'RECIPE_VAULT', 'BREW_FEED');
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS coffee_communities (
    id UUID PRIMARY KEY,
    handle_slug VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(90) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    avatar_url TEXT,
    banner_url TEXT,
    privacy_level community_privacy_kind NOT NULL DEFAULT 'PUBLIC',
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    focus_tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    taste_feature_vector REAL[] NOT NULL DEFAULT ARRAY[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]::REAL[],
    members_count INT NOT NULL DEFAULT 1,
    threads_count INT NOT NULL DEFAULT 0,
    shared_recipes_count INT NOT NULL DEFAULT 0,
    total_club_xp BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_communities_public_discover
    ON coffee_communities (members_count DESC, total_club_xp DESC)
    WHERE privacy_level <> 'SECRET_VAULT';

CREATE TABLE IF NOT EXISTS community_invite_secrets (
    id UUID PRIMARY KEY,
    community_id UUID NOT NULL REFERENCES coffee_communities(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    secret_code_display VARCHAR(24) NOT NULL UNIQUE,
    secret_code_sha256 BYTEA NOT NULL UNIQUE,
    invite_url_token VARCHAR(64) NOT NULL UNIQUE,
    role_granted community_member_role NOT NULL DEFAULT 'BARISTA_MEMBER',
    max_uses INT,
    used_count INT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_invite_secrets_lookup
    ON community_invite_secrets (secret_code_sha256)
    WHERE is_revoked = FALSE;

CREATE INDEX IF NOT EXISTS idx_invite_url_token_lookup
    ON community_invite_secrets (invite_url_token)
    WHERE is_revoked = FALSE;

CREATE TABLE IF NOT EXISTS community_members (
    community_id UUID NOT NULL REFERENCES coffee_communities(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role community_member_role NOT NULL DEFAULT 'BARISTA_MEMBER',
    club_xp_contributed BIGINT NOT NULL DEFAULT 0,
    brews_shared_count INT NOT NULL DEFAULT 0,
    is_notifications_muted BOOLEAN NOT NULL DEFAULT FALSE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (community_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_comm_members_user_clubs
    ON community_members (user_id, joined_at DESC);

CREATE INDEX IF NOT EXISTS idx_comm_members_leaderboard
    ON community_members (community_id, club_xp_contributed DESC);

CREATE TABLE IF NOT EXISTS community_channels (
    id UUID PRIMARY KEY,
    community_id UUID NOT NULL REFERENCES coffee_communities(id) ON DELETE CASCADE,
    name VARCHAR(60) NOT NULL,
    description VARCHAR(200),
    kind community_channel_kind NOT NULL DEFAULT 'REALTIME_CHAT',
    linked_conversation_id UUID REFERENCES conversations(id) ON DELETE SET NULL,
    display_order SMALLINT NOT NULL DEFAULT 0,
    is_readonly_announcement BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_comm_channels_order
    ON community_channels (community_id, display_order ASC);

ALTER TABLE social_threads
    ADD COLUMN IF NOT EXISTS community_id UUID REFERENCES coffee_communities(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS trending_gravity_score DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    ADD COLUMN IF NOT EXISTS taste_feature_vector REAL[] NOT NULL DEFAULT ARRAY[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]::REAL[];

CREATE INDEX IF NOT EXISTS idx_threads_community_feed
    ON social_threads (community_id, is_pinned_on_profile DESC, id DESC)
    WHERE is_deleted = FALSE AND community_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_threads_community_recipe_vault
    ON social_threads (community_id, try_recipe_count DESC, id DESC)
    WHERE is_deleted = FALSE AND community_id IS NOT NULL AND attachment_type = 'RECIPE_CARD';

CREATE INDEX IF NOT EXISTS idx_threads_trending_gravity
    ON social_threads (trending_gravity_score DESC, id DESC)
    WHERE is_deleted = FALSE AND depth_level = 0;

CREATE TABLE IF NOT EXISTS user_taste_affinity_profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    affinity_vector REAL[] NOT NULL DEFAULT ARRAY[0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1,0.1]::REAL[],
    favorite_origins JSONB NOT NULL DEFAULT '[]'::jsonb,
    favorite_processes JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
