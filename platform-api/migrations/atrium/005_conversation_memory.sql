CREATE TABLE IF NOT EXISTS atrium_conversation_messages (
    id BIGSERIAL PRIMARY KEY,
    guild_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('member', 'atrium')),
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_atrium_conversation_messages_lookup
    ON atrium_conversation_messages (guild_id, member_id, id DESC);
