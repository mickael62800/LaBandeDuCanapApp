-- Sessions vocales individuelles pour statistiques detaillees par salon
CREATE TABLE IF NOT EXISTS voice_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    channel_name    TEXT NOT NULL DEFAULT '',
    duration_secs   BIGINT NOT NULL DEFAULT 0,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_voice_sessions_guild ON voice_sessions (guild_id);
CREATE INDEX idx_voice_sessions_channel ON voice_sessions (channel_id);
CREATE INDEX idx_voice_sessions_user ON voice_sessions (guild_id, user_id);
CREATE INDEX idx_voice_sessions_started ON voice_sessions (started_at);
