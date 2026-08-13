-- Panneaux de jeux : embed Discord avec reactions par emoji.
-- Les users cliquent une reaction pour s'abonner/desabonner a un jeu.

-- Ajoute emoji + categorie aux jeux (nullable pour retrocompat).
ALTER TABLE games ADD COLUMN IF NOT EXISTS emoji TEXT;
ALTER TABLE games ADD COLUMN IF NOT EXISTS category TEXT;

-- Un panneau par (guild, categorie). Une categorie NULL est traitee
-- comme une chaine vide pour unicite (jeux "sans categorie").
CREATE TABLE IF NOT EXISTS game_panels (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    channel_id  TEXT NOT NULL,
    message_id  TEXT NOT NULL,
    category    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_game_panels_guild_cat
    ON game_panels (guild_id, COALESCE(category, ''));
CREATE INDEX IF NOT EXISTS idx_game_panels_message
    ON game_panels (guild_id, message_id);
