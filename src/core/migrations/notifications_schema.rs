pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'mobile_platform_kind') THEN
                CREATE TYPE mobile_platform_kind AS ENUM ('ANDROID', 'IOS', 'WEB');
            END IF;
        END
        $$"#,
        r#"CREATE TABLE IF NOT EXISTS user_device_tokens (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            device_installation_id VARCHAR(120) NOT NULL,
            platform mobile_platform_kind NOT NULL DEFAULT 'ANDROID',
            fcm_token TEXT NOT NULL UNIQUE,
            apns_voip_or_live_activity_token TEXT,
            app_version VARCHAR(30) NOT NULL DEFAULT '1.0.0',
            device_model VARCHAR(100),
            is_kiosk_aod_active BOOLEAN NOT NULL DEFAULT FALSE,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            CONSTRAINT uq_user_device_install UNIQUE (device_installation_id)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_device_tokens_user_active ON user_device_tokens (user_id) WHERE is_active = TRUE AND is_kiosk_aod_active = FALSE",
        r#"CREATE TABLE IF NOT EXISTS user_notification_preferences (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            enable_chat_messages BOOLEAN NOT NULL DEFAULT TRUE,
            enable_follow_follback BOOLEAN NOT NULL DEFAULT TRUE,
            enable_thread_replies BOOLEAN NOT NULL DEFAULT TRUE,
            enable_likes_and_recipe_tries BOOLEAN NOT NULL DEFAULT TRUE,
            enable_live_brew_and_streak BOOLEAN NOT NULL DEFAULT TRUE,
            enable_transactional_billing BOOLEAN NOT NULL DEFAULT TRUE,
            quiet_hours_start_minute SMALLINT,
            quiet_hours_end_minute SMALLINT,
            timezone VARCHAR(64) NOT NULL DEFAULT 'Asia/Jakarta',
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS notification_inbox (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            coalesce_group_key VARCHAR(160),
            category VARCHAR(30) NOT NULL,
            event_type VARCHAR(40) NOT NULL,
            title VARCHAR(160) NOT NULL,
            body TEXT NOT NULL,
            deep_link_path TEXT NOT NULL,
            action_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
            actors_preview JSONB NOT NULL DEFAULT '[]'::jsonb,
            aggregated_count INT NOT NULL DEFAULT 1,
            is_read BOOLEAN NOT NULL DEFAULT FALSE,
            read_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"DO $$
        BEGIN
            IF EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'notification_inbox'
                  AND column_name = 'recipient_id'
            ) AND NOT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'notification_inbox'
                  AND column_name = 'user_id'
            ) THEN
                ALTER TABLE notification_inbox RENAME COLUMN recipient_id TO user_id;
            END IF;
        END
        $$"#,
        "DROP INDEX IF EXISTS idx_notif_inbox_unread_coalesce",
        "CREATE INDEX IF NOT EXISTS idx_notif_inbox_user_cursor ON notification_inbox (user_id, updated_at DESC, id DESC)",
        "CREATE INDEX IF NOT EXISTS idx_notif_inbox_unread_user ON notification_inbox (user_id, id DESC) WHERE is_read = FALSE",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_notif_inbox_coalesce_upsert ON notification_inbox (user_id, coalesce_group_key) WHERE coalesce_group_key IS NOT NULL",
    ]
}
