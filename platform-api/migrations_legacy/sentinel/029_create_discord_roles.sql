-- Stockage des roles Discord synchronises par le community-bot
CREATE TABLE IF NOT EXISTS discord_roles (
    id              TEXT NOT NULL,
    guild_id        TEXT NOT NULL,
    name            TEXT NOT NULL,
    color           INTEGER NOT NULL DEFAULT 0,
    position        INTEGER NOT NULL DEFAULT 0,
    permissions     TEXT NOT NULL DEFAULT '0',
    mentionable     BOOLEAN NOT NULL DEFAULT FALSE,
    managed         BOOLEAN NOT NULL DEFAULT FALSE,
    icon            TEXT,
    member_count    INTEGER NOT NULL DEFAULT 0,
    synced_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, id)
);

CREATE INDEX IF NOT EXISTS idx_discord_roles_guild ON discord_roles(guild_id, position DESC);
