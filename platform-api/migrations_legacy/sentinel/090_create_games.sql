-- Jeux mentionnables par serveur.
-- Les joueurs s'inscrivent a des jeux et sont ping quand quelqu'un ecrit #NomDuJeu.
CREATE TABLE IF NOT EXISTS games (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    game_name   TEXT NOT NULL,
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_games_guild_name ON games (guild_id, LOWER(game_name));
CREATE INDEX idx_games_guild ON games (guild_id);

-- Inscriptions des joueurs aux jeux.
CREATE TABLE IF NOT EXISTS game_subscriptions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    game_id     UUID NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (game_id, user_id)
);

CREATE INDEX idx_game_subs_game ON game_subscriptions (game_id);
CREATE INDEX idx_game_subs_user ON game_subscriptions (guild_id, user_id);
