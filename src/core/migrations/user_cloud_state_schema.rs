pub fn statements() -> &'static [&'static str] {
    &[
        r#"CREATE TABLE IF NOT EXISTS user_cloud_state (
            user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
            total_exp INT NOT NULL DEFAULT 0,
            brews_count INT NOT NULL DEFAULT 0,
            toolkit_count INT NOT NULL DEFAULT 0,
            cupping_count INT NOT NULL DEFAULT 0,
            streak_days INT NOT NULL DEFAULT 1,
            last_activity_date VARCHAR(16),
            brew_history_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            brew_journal_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            favorite_recipe_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            custom_recipes_json JSONB NOT NULL DEFAULT '[]'::jsonb,
            deleted_default_ids JSONB NOT NULL DEFAULT '[]'::jsonb,
            state_version BIGINT NOT NULL DEFAULT 1,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_user_cloud_state_updated ON user_cloud_state (updated_at DESC)",
    ]
}
