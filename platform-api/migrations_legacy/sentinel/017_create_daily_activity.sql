-- Snapshots d'activite quotidienne pour les graphiques du dashboard
CREATE TABLE IF NOT EXISTS daily_activity (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    day DATE NOT NULL,
    messages BIGINT NOT NULL DEFAULT 0,
    voice_minutes BIGINT NOT NULL DEFAULT 0,
    active_members INTEGER NOT NULL DEFAULT 0,
    new_members INTEGER NOT NULL DEFAULT 0,
    infractions INTEGER NOT NULL DEFAULT 0,
    warns INTEGER NOT NULL DEFAULT 0,
    mutes INTEGER NOT NULL DEFAULT 0,
    bans INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT uq_daily_activity_guild_day UNIQUE (guild_id, day)
);

CREATE INDEX idx_daily_activity_guild_day ON daily_activity (guild_id, day DESC);
