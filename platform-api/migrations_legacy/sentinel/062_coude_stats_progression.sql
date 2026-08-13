-- Ajout du systeme de progression (stats, niveaux, XP) au jeu Coup de Coude.

ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS level INT NOT NULL DEFAULT 1;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS xp BIGINT NOT NULL DEFAULT 0;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS atk INT NOT NULL DEFAULT 0;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS def INT NOT NULL DEFAULT 0;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS stat_points INT NOT NULL DEFAULT 0;
ALTER TABLE coude_players ADD COLUMN IF NOT EXISTS title TEXT NOT NULL DEFAULT 'Debutant';

-- Index pour le leaderboard par niveau
CREATE INDEX IF NOT EXISTS idx_coude_players_level ON coude_players(guild_id, level DESC, xp DESC);
