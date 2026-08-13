-- S1/S4 securite tickets : ajoute `guild_id` a la table tickets pour pouvoir
-- scoper les endpoints HTTP par guild du caller (RBAC Moderator+).
--
-- Avant cette migration, `tickets` ne portait que `server` (= nom de guild),
-- ce qui empechait toute autorisation fiable par guild : l'API HTTP fuyait
-- TOUS les tickets + transcripts cross-guild. On ajoute un vrai `guild_id`.
--
-- Nullable pour permettre un backfill best-effort des lignes existantes.
ALTER TABLE tickets ADD COLUMN IF NOT EXISTS guild_id TEXT;

-- Backfill best-effort : on resout le guild_id depuis le nom de serveur.
-- Si les noms de guild ne sont pas uniques, c'est best-effort (acceptable) :
-- les lignes ambigües ou non resolues restent NULL. Les lignes NULL sont
-- traitees comme "acces web refuse" cote handler (fail-closed), seul le bot
-- (gRPC, de confiance) peut encore les atteindre.
UPDATE tickets t
SET guild_id = g.guild_id
FROM guilds g
WHERE t.guild_id IS NULL
  AND t.server = g.name;

CREATE INDEX IF NOT EXISTS idx_tickets_guild_id ON tickets (guild_id);
