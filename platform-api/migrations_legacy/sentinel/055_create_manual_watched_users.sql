-- Table pour les utilisateurs mis en surveillance manuellement (sans infraction)
CREATE TABLE IF NOT EXISTS manual_watched_users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    username    TEXT NOT NULL,
    reason      TEXT NOT NULL DEFAULT '',
    added_by    TEXT NOT NULL DEFAULT 'desktop',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id)
);

CREATE INDEX idx_manual_watched_guild ON manual_watched_users (guild_id);
