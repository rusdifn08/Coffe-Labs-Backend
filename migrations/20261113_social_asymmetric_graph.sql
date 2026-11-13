-- ============================================================================
-- Coffee Labs Social Coffee (Modul 3.3): Asymmetric Social Graph
-- Follow, Follback, Private Follow Requests & Bidirectional Mutual Engine
-- ============================================================================

-- 1. Enum Status Relasi Follow (Mendukung Akun Publik & Akun Privat)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'follow_edge_status') THEN
        CREATE TYPE follow_edge_status AS ENUM ('ACCEPTED', 'PENDING');
    END IF;
END
$$;

-- 2. Tambahkan Kolom Pengaturan Privasi pada Tabel Proyeksi Profil (Modul 3.2)
ALTER TABLE user_profile_projections
    ADD COLUMN IF NOT EXISTS is_private_account BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS pending_follow_requests_count INT NOT NULL DEFAULT 0;

-- 3. Tabel Tunggal Asymmetric Directed Graph (`user_follows`)
CREATE TABLE IF NOT EXISTS user_follows (
    follower_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status follow_edge_status NOT NULL DEFAULT 'ACCEPTED',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ,

    PRIMARY KEY (follower_id, following_id),
    CONSTRAINT chk_user_follows_no_self CHECK (follower_id <> following_id)
);

-- 4. Backfill data lama dari tabel `social_follows` (Modul 3.2) bila ada
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'social_follows'
    ) THEN
        INSERT INTO user_follows (follower_id, following_id, status, created_at, accepted_at)
        SELECT follower_id, following_id, 'ACCEPTED'::follow_edge_status, created_at, created_at
        FROM social_follows
        ON CONFLICT (follower_id, following_id) DO NOTHING;
    END IF;
END
$$;

-- 5. INDEKS TERBALIK & INDEKS KEYSET PAGINATION BERKECEPATAN TINGGI
CREATE INDEX IF NOT EXISTS idx_user_follows_reverse_lookup
    ON user_follows (following_id, follower_id, status);

CREATE INDEX IF NOT EXISTS idx_user_follows_followers_list
    ON user_follows (following_id, created_at DESC, follower_id)
    WHERE status = 'ACCEPTED';

CREATE INDEX IF NOT EXISTS idx_user_follows_following_list
    ON user_follows (follower_id, created_at DESC, following_id)
    WHERE status = 'ACCEPTED';

CREATE INDEX IF NOT EXISTS idx_user_follows_pending_requests
    ON user_follows (following_id, created_at DESC, follower_id)
    WHERE status = 'PENDING';
