/// Idempotent runtime DDL statements for the Social Coffee real-time chat engine.
pub fn statements() -> &'static [&'static str] {
    &[
        r#"DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'conversation_kind') THEN
                CREATE TYPE conversation_kind AS ENUM ('DIRECT', 'GROUP');
            END IF;
        END
        $$"#,
        r#"CREATE TABLE IF NOT EXISTS conversations (
            id UUID PRIMARY KEY,
            kind conversation_kind NOT NULL DEFAULT 'DIRECT',
            direct_pair_key VARCHAR(80) UNIQUE,
            title VARCHAR(120),
            avatar_url TEXT,
            last_seq_id BIGINT NOT NULL DEFAULT 0,
            last_message_preview VARCHAR(250),
            last_message_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_conversations_last_msg_at ON conversations (last_message_at DESC NULLS LAST)",
        r#"CREATE TABLE IF NOT EXISTS conversation_participants (
            conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role VARCHAR(20) NOT NULL DEFAULT 'MEMBER',
            last_delivered_seq_id BIGINT NOT NULL DEFAULT 0,
            last_read_seq_id BIGINT NOT NULL DEFAULT 0,
            is_muted BOOLEAN NOT NULL DEFAULT FALSE,
            joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (conversation_id, user_id)
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_conv_participants_user ON conversation_participants (user_id, conversation_id)",
        r#"CREATE TABLE IF NOT EXISTS chat_messages (
            id UUID NOT NULL,
            conversation_id UUID NOT NULL,
            seq_id BIGINT NOT NULL,
            sender_id UUID NOT NULL,
            client_msg_id UUID NOT NULL,
            reply_to_msg_id UUID,
            message_type VARCHAR(30) NOT NULL DEFAULT 'TEXT',
            body_text TEXT,
            attachment_payload JSONB,
            is_edited BOOLEAN NOT NULL DEFAULT FALSE,
            is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (conversation_id, seq_id),
            CONSTRAINT uq_chat_msg_client_id UNIQUE (conversation_id, client_msg_id)
        ) PARTITION BY HASH (conversation_id)"#,
        "CREATE TABLE IF NOT EXISTS chat_messages_p0 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 0)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p1 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 1)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p2 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 2)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p3 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 3)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p4 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 4)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p5 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 5)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p6 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 6)",
        "CREATE TABLE IF NOT EXISTS chat_messages_p7 PARTITION OF chat_messages FOR VALUES WITH (MODULUS 8, REMAINDER 7)",
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_conv_created ON chat_messages (conversation_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_id_lookup ON chat_messages (conversation_id, id)",
    ]
}
