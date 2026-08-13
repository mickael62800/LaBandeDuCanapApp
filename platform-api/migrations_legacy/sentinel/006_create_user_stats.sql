CREATE TABLE IF NOT EXISTS user_stats (
    id UUID PRIMARY KEY,
    guild_id VARCHAR NOT NULL,
    user_id VARCHAR NOT NULL,
    username VARCHAR NOT NULL DEFAULT '',
    message_count BIGINT NOT NULL DEFAULT 0,
    voice_seconds BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_user_stats_guild_user UNIQUE (guild_id, user_id)
);

CREATE INDEX idx_user_stats_guild ON user_stats (guild_id);
CREATE INDEX idx_user_stats_guild_user ON user_stats (guild_id, user_id);
