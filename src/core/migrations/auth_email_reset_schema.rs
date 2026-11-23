pub fn statements() -> &'static [&'static str] {
    &[
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS email VARCHAR(160)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_lower_unique ON users (LOWER(email)) WHERE email IS NOT NULL AND TRIM(email) <> ''",
        r#"CREATE TABLE IF NOT EXISTS password_reset_tokens (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            email VARCHAR(160) NOT NULL,
            code_hash VARCHAR(128) NOT NULL,
            attempts INT NOT NULL DEFAULT 0,
            expires_at TIMESTAMPTZ NOT NULL,
            used_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_pwd_reset_user_created ON password_reset_tokens (user_id, created_at DESC)",
    ]
}
