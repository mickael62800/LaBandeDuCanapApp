-- Game Portal — "evenements de serveur" : au lancement d'un serveur, le bot
-- cree des salons Discord (texte + vocal prives), ping le role du jeu, gere
-- les inscriptions, et revele l'IP a une echeance programmee.

-- 1) Reglages par (guild, template) : les templates sont un catalogue GLOBAL,
--    mais le role a pinguer (@Minecraft) est propre a chaque serveur Discord.
CREATE TABLE IF NOT EXISTS game_template_settings (
    guild_id        TEXT NOT NULL,
    template_slug   TEXT NOT NULL,
    discord_role_id TEXT,            -- role a pinguer pour ce jeu sur cette guild
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, template_slug)
);

-- 2) Colonnes de session sur game_servers (salons crees + revelation IP).
ALTER TABLE game_servers
    ADD COLUMN IF NOT EXISTS text_channel_id  TEXT,
    ADD COLUMN IF NOT EXISTS voice_channel_id TEXT,
    ADD COLUMN IF NOT EXISTS ip_reveal_at     TIMESTAMPTZ,   -- NULL = pas de revelation programmee
    ADD COLUMN IF NOT EXISTS ip_revealed      BOOLEAN NOT NULL DEFAULT false;

-- Scan du job de revelation : bans/serveurs dont l'IP doit etre revelee.
CREATE INDEX IF NOT EXISTS idx_game_servers_ip_reveal
    ON game_servers (ip_reveal_at)
    WHERE ip_revealed = false AND ip_reveal_at IS NOT NULL AND deleted_at IS NULL;

-- 3) Inscriptions des joueurs a une session (bouton "Je m'inscris").
CREATE TABLE IF NOT EXISTS game_session_registrations (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id     UUID NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    user_id       TEXT NOT NULL,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (server_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_game_session_reg_server
    ON game_session_registrations (server_id);

-- 4) Nouveaux reglages globaux game-portal (dashboard).
UPDATE bot_definitions
SET config_schema = config_schema || '[
    {"key": "session_category_id", "label": "Categorie des salons de session de jeu", "type": "category", "required": false},
    {"key": "ip_reveal_default_days", "label": "Delai avant revelation de l IP (jours)", "type": "number", "required": false, "default": "7"},
    {"key": "session_daily_ping_enabled", "label": "Ping quotidien du role pendant l attente", "type": "boolean", "required": false, "default": "false"},
    {"key": "session_daily_ping_hour", "label": "Heure du ping quotidien (0-23, UTC)", "type": "number", "required": false, "default": "18"}
]'::jsonb
WHERE bot_name = 'game-portal'
  AND NOT (config_schema @> '[{"key": "session_category_id"}]'::jsonb);
