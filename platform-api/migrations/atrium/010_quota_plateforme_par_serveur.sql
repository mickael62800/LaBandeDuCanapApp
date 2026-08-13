-- P3 - Le compteur de quota « plateforme » devient PAR SERVEUR.
--
-- `global_daily_limit` est configurable par serveur (`bot_guild_config`) et
-- presente comme un plafond par serveur dans l'administration. La table ne
-- respectait pourtant pas cette semantique : une unique ligne par date, toutes
-- guildes confondues, qui verrouillait et serialisait les appels de tous les
-- serveurs entre eux. On ajoute `guild_id` a la cle pour que chaque serveur ait
-- son propre compteur et son propre verrou.
ALTER TABLE atrium_ai_usage_global
    ADD COLUMN IF NOT EXISTS guild_id TEXT NOT NULL DEFAULT '';

ALTER TABLE atrium_ai_usage_global DROP CONSTRAINT IF EXISTS atrium_ai_usage_global_pkey;
ALTER TABLE atrium_ai_usage_global ADD PRIMARY KEY (usage_date, guild_id);

-- Le defaut n'existait que pour peupler la colonne sur les lignes deja en base
-- (elles portent alors guild_id = '' et vieilliront d'elles-memes). Les
-- insertions applicatives fournissent toujours le guild_id explicitement.
ALTER TABLE atrium_ai_usage_global ALTER COLUMN guild_id DROP DEFAULT;
