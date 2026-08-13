CREATE TABLE IF NOT EXISTS infractions (
    id         UUID PRIMARY KEY,
    guild_id   TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    user_id    TEXT NOT NULL,
    username   TEXT NOT NULL,
    message_id TEXT NOT NULL,
    content    TEXT NOT NULL,
    flags      JSONB NOT NULL,
    score      DOUBLE PRECISION NOT NULL,
    action     TEXT NOT NULL,
    reason     TEXT NOT NULL,
    duration   BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_infractions_guild ON infractions (guild_id);
CREATE INDEX IF NOT EXISTS idx_infractions_user  ON infractions (guild_id, user_id);
