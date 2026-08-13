-- Table de cooldowns pour le jeu Coup de Coude
CREATE TABLE IF NOT EXISTS coude_cooldowns (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    action      TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    UNIQUE(guild_id, user_id, action)
);

CREATE INDEX IF NOT EXISTS idx_coude_cooldowns_lookup ON coude_cooldowns(guild_id, user_id, action);
