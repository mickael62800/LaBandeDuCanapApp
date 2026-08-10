-- Config Atrium par serveur : sortie des variables d'environnement.
--
-- POURQUOI
--
-- Les quotas d'Atrium (limite par membre, cooldown, plafond global) etaient
-- lus une seule fois au demarrage depuis l'environnement du processus. Deux
-- consequences : la meme valeur s'appliquait a tous les serveurs, et changer
-- un plafond imposait de redemarrer le conteneur. La regle du depot est
-- l'inverse : un reglage se declare dans `bot_definitions.config_schema` et se
-- lit dans `bot_guild_config`, les variables d'environnement ne servant que de
-- valeur de repli au demarrage.
--
-- Ces deux tables sont repliquees ici plutot que lues chez Sentinel : Atrium a
-- sa propre base logique et n'a aucun acces a `discord_sentinel`. C'est
-- exactement le choix fait par Nexus (nexus-api/migrations/007_game_portal.sql).

-- ── Config bot par guild ──

CREATE TABLE IF NOT EXISTS bot_definitions (
    bot_name character varying(50) PRIMARY KEY,
    display_name character varying(100) NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    config_schema jsonb DEFAULT '[]'::jsonb NOT NULL
);

CREATE TABLE IF NOT EXISTS bot_guild_config (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    bot_name character varying(50) NOT NULL,
    config_key character varying(100) NOT NULL,
    config_value text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT bot_guild_config_guild_id_bot_name_config_key_key
        UNIQUE (guild_id, bot_name, config_key)
);

CREATE INDEX IF NOT EXISTS idx_bot_guild_config_bot ON bot_guild_config USING btree (guild_id, bot_name);
CREATE INDEX IF NOT EXISTS idx_bot_guild_config_guild ON bot_guild_config USING btree (guild_id);

-- ── Declaration du bot Atrium ──
--
-- Les valeurs par defaut reprennent celles du code (`AppConfig::from_env`),
-- pour qu'une installation sans reglage explicite se comporte comme avant.

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema) VALUES (
    'atrium-bot',
    'Accueil IA',
    'Accueille les nouveaux membres et repond a leurs questions a partir de la base de connaissances du serveur.',
    '[
      {"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false,
       "description": "Desactive, le bot repond qu il est hors service au lieu d appeler le modele. Aucun quota n est consomme."},
      {"key": "user_daily_limit", "type": "number", "unit": "requetes", "min": 0, "max": 10000, "label": "Requetes par membre et par jour", "default": "30", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "0 = illimite. Au-dela, le membre recoit un message d attente jusqu au lendemain."},
      {"key": "user_cooldown_secs", "type": "number", "unit": "s", "min": 0, "max": 3600, "label": "Delai entre deux questions d un meme membre", "default": "10", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "0 = pas de delai. Ne s applique qu aux questions, pas au message d accueil automatique."},
      {"key": "global_daily_limit", "type": "number", "unit": "requetes", "min": 0, "max": 100000, "label": "Plafond quotidien du serveur", "default": "500", "required": false,
       "depends_on": {"key": "enabled", "equals": "true"},
       "description": "0 = illimite. Protege la facture du fournisseur de modele."}
    ]'::jsonb
) ON CONFLICT (bot_name) DO NOTHING;

-- ── Reprise de l'etat existant ──
--
-- Le flag `enabled` suit desormais la semantique de reference du depot :
-- CLE ABSENTE = MODULE DESACTIVE (fail-closed). Sans reprise, appliquer cette
-- migration couperait donc l'accueil sur un serveur qui n'a jamais touche au
-- reglage — il etait implicitement actif.
--
-- On ecrit donc une valeur EXPLICITE pour tout serveur qu'Atrium a reellement
-- servi, en balayant les cinq tables qui portent un `guild_id`. Un serveur qui
-- a des documents indexes, de la memoire de conversation ou de la consommation
-- est un serveur en service : il reste actif.
--
-- `atrium_guild_settings` fait foi quand elle a une ligne (elle porte le choix
-- explicite d'un administrateur), et reste en place ensuite : elle conserve
-- `updated_by`/`updated_at`, donc la trace de QUI a bascule l'interrupteur.

INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
SELECT guild_id, 'atrium-bot', 'enabled', CASE WHEN enabled THEN 'true' ELSE 'false' END
FROM atrium_guild_settings
WHERE char_length(guild_id) <= 20
ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;

INSERT INTO bot_guild_config (guild_id, bot_name, config_key, config_value)
SELECT DISTINCT guild_id, 'atrium-bot', 'enabled', 'true'
FROM (
    SELECT guild_id FROM atrium_ai_usage_users
    UNION SELECT guild_id FROM atrium_knowledge_documents
    UNION SELECT guild_id FROM atrium_conversation_messages
    UNION SELECT guild_id FROM atrium_server_summaries
) AS servis
WHERE char_length(guild_id) <= 20
ON CONFLICT (guild_id, bot_name, config_key) DO NOTHING;
