CREATE TABLE IF NOT EXISTS atrium_server_summaries (
    id UUID PRIMARY KEY,
    guild_id VARCHAR(64) NOT NULL,
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atrium_summaries_guild ON atrium_server_summaries(guild_id, created_at DESC);
