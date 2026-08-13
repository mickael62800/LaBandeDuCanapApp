-- Coup de Coude v2 : systeme saisonnier

CREATE TABLE IF NOT EXISTS coude_seasons (
    id SERIAL PRIMARY KEY,
    guild_id TEXT NOT NULL,
    season_number INTEGER NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    champion_id TEXT,
    champion_name TEXT,
    UNIQUE(guild_id, season_number)
);

CREATE TABLE IF NOT EXISTS coude_season_titles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    season_number INTEGER NOT NULL,
    title_key TEXT NOT NULL,   -- 'champion', 'best_fighter', 'thief_king', 'chaos_king', 'casino_legend'
    title_label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_season_titles_user ON coude_season_titles (guild_id, user_id);
