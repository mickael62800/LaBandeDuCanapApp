-- Table de log du chaos quotidien pour le jeu Coup de Coude
CREATE TABLE IF NOT EXISTS coude_daily_chaos (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    loser_id    TEXT NOT NULL,
    loser_name  TEXT NOT NULL,
    winner_id   TEXT NOT NULL,
    winner_name TEXT NOT NULL,
    amount      BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_coude_daily_chaos_guild ON coude_daily_chaos(guild_id, created_at DESC);
