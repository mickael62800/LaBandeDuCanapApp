CREATE TABLE IF NOT EXISTS security_events (
    id          UUID PRIMARY KEY,
    guild_id    TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    severity    TEXT NOT NULL,
    description TEXT NOT NULL,
    user_ids    JSONB NOT NULL DEFAULT '[]',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_security_events_guild ON security_events (guild_id);
CREATE INDEX IF NOT EXISTS idx_security_events_type ON security_events (event_type);
