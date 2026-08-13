-- 007_game_portal.sql — reintegration du systeme game-portal dans Nexus.
--
-- Portage des tables supprimees de sentinel-api/migrations/001_init.sql au
-- commit 0e5549a7. Schema identique a l'original, aux exceptions pres :
--   - pas de FK vers les tables sentinel absentes de la base nexus (guilds…) ;
--   - tables bot_definitions / bot_guild_config incluses car nexus-api en a
--     besoin pour la config game-portal par guild (BotConfigRepository).

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

-- ── Catalogue des jeux mentionnables + panneaux Discord ──

CREATE TABLE IF NOT EXISTS games (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    game_name text NOT NULL,
    created_by text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    emoji text,
    category text,
    role_id text
);

CREATE TABLE IF NOT EXISTS game_panels (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id text NOT NULL,
    channel_id text NOT NULL,
    message_id text NOT NULL,
    category text,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_game_panels_message ON game_panels USING btree (guild_id, message_id);

-- ── Templates de serveurs de jeu (images Docker) ──

CREATE TABLE IF NOT EXISTS game_templates (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    slug character varying(64) NOT NULL UNIQUE,
    name character varying(128) NOT NULL,
    description text,
    image character varying(255) NOT NULL,
    category character varying(64),
    icon character varying(16),
    accent_color character varying(8),
    container_port integer NOT NULL,
    default_memory_mb integer DEFAULT 2048 NOT NULL,
    min_memory_mb integer DEFAULT 512 NOT NULL,
    max_memory_mb integer DEFAULT 8192 NOT NULL,
    default_env jsonb DEFAULT '{}'::jsonb NOT NULL,
    config_schema jsonb DEFAULT '[]'::jsonb NOT NULL,
    supports_rcon boolean DEFAULT false NOT NULL,
    supports_mods boolean DEFAULT false NOT NULL,
    idle_shutdown_days integer DEFAULT 7 NOT NULL,
    deleted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    port_protocol character varying(8) DEFAULT 'tcp'::character varying NOT NULL,
    cover_image_url character varying(512),
    volume_path character varying(255) DEFAULT '/data'::character varying NOT NULL,
    run_as_root boolean DEFAULT false NOT NULL,
    init_files jsonb DEFAULT '[]'::jsonb NOT NULL,
    command_template text,
    CONSTRAINT chk_game_templates_memory CHECK (((default_memory_mb >= min_memory_mb) AND (default_memory_mb <= max_memory_mb) AND (min_memory_mb >= 256) AND (max_memory_mb <= 32768))),
    CONSTRAINT chk_game_templates_port CHECK (((container_port >= 1) AND (container_port <= 65535))),
    CONSTRAINT chk_game_templates_protocol CHECK (((port_protocol)::text = ANY ((ARRAY['tcp'::character varying, 'udp'::character varying])::text[])))
);

CREATE TABLE IF NOT EXISTS game_template_settings (
    guild_id text NOT NULL,
    template_slug text NOT NULL,
    discord_role_id text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    PRIMARY KEY (guild_id, template_slug)
);

-- ── Serveurs de jeu instancies ──

CREATE TABLE IF NOT EXISTS game_servers (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    guild_id character varying(20) NOT NULL,
    template_id uuid NOT NULL REFERENCES game_templates(id),
    name character varying(64) NOT NULL,
    status character varying(20) DEFAULT 'created'::character varying NOT NULL,
    container_id character varying(64),
    container_name character varying(64),
    host_port integer,
    rcon_port integer,
    rcon_password text,
    volume_name character varying(64),
    allocated_memory_mb integer NOT NULL,
    owner_user_id character varying(20) NOT NULL,
    idle_shutdown_days integer,
    last_active_at timestamp with time zone,
    last_player_count integer DEFAULT 0 NOT NULL,
    last_error text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    started_at timestamp with time zone,
    stopped_at timestamp with time zone,
    deleted_at timestamp with time zone,
    restart_attempts integer DEFAULT 0 NOT NULL,
    last_restart_at timestamp with time zone,
    text_channel_id text,
    voice_channel_id text,
    ip_reveal_at timestamp with time zone,
    ip_revealed boolean DEFAULT false NOT NULL,
    last_daily_ping_at timestamp with time zone,
    CONSTRAINT chk_game_servers_host_port CHECK (((host_port IS NULL) OR ((host_port >= 1024) AND (host_port <= 65535)))),
    CONSTRAINT chk_game_servers_memory CHECK (((allocated_memory_mb >= 256) AND (allocated_memory_mb <= 32768))),
    CONSTRAINT chk_game_servers_name CHECK ((((char_length((name)::text) >= 1) AND (char_length((name)::text) <= 64)) AND ((name)::text ~ '^[a-zA-Z0-9 _\-]+$'::text))),
    CONSTRAINT chk_game_servers_rcon_port CHECK (((rcon_port IS NULL) OR ((rcon_port >= 1024) AND (rcon_port <= 65535)))),
    CONSTRAINT chk_game_servers_status CHECK (((status)::text = ANY ((ARRAY['created'::character varying, 'starting'::character varying, 'running'::character varying, 'stopping'::character varying, 'stopped'::character varying, 'error'::character varying, 'deleted'::character varying])::text[])))
);

CREATE INDEX IF NOT EXISTS idx_game_servers_guild ON game_servers USING btree (guild_id) WHERE (deleted_at IS NULL);
CREATE INDEX IF NOT EXISTS idx_game_servers_ip_reveal ON game_servers USING btree (ip_reveal_at) WHERE ((ip_revealed = false) AND (ip_reveal_at IS NOT NULL) AND (deleted_at IS NULL));
CREATE INDEX IF NOT EXISTS idx_game_servers_last_active ON game_servers USING btree (last_active_at) WHERE ((deleted_at IS NULL) AND ((status)::text = 'running'::text));

CREATE TABLE IF NOT EXISTS game_server_configs (
    server_id uuid NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    config_key character varying(64) NOT NULL,
    config_value text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by character varying(20),
    PRIMARY KEY (server_id, config_key),
    CONSTRAINT chk_game_server_configs_key CHECK ((((char_length((config_key)::text) >= 1) AND (char_length((config_key)::text) <= 64)) AND ((config_key)::text ~ '^[A-Z][A-Z0-9_]*$'::text)))
);

-- ── Sessions joueurs + inscriptions Discord ──

CREATE TABLE IF NOT EXISTS game_player_sessions (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    server_id uuid NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    player_name character varying(64) NOT NULL,
    joined_at timestamp with time zone DEFAULT now() NOT NULL,
    left_at timestamp with time zone,
    duration_seconds integer GENERATED ALWAYS AS (
        CASE
            WHEN (left_at IS NOT NULL) THEN (EXTRACT(epoch FROM (left_at - joined_at)))::integer
            ELSE NULL::integer
        END) STORED
);

CREATE INDEX IF NOT EXISTS idx_game_player_sessions_active ON game_player_sessions USING btree (server_id, player_name) WHERE (left_at IS NULL);
CREATE INDEX IF NOT EXISTS idx_game_player_sessions_server ON game_player_sessions USING btree (server_id, joined_at DESC);

CREATE TABLE IF NOT EXISTS game_session_registrations (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    server_id uuid NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    user_id text NOT NULL,
    registered_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT game_session_registrations_server_id_user_id_key UNIQUE (server_id, user_id)
);

-- ── Audit + backups ──

CREATE TABLE IF NOT EXISTS game_audit_log (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    server_id uuid REFERENCES game_servers(id) ON DELETE SET NULL,
    guild_id character varying(20) NOT NULL,
    actor_user_id character varying(20),
    action character varying(32) NOT NULL,
    details jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_game_audit_log_guild ON game_audit_log USING btree (guild_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_game_audit_log_server ON game_audit_log USING btree (server_id, created_at DESC);

CREATE TABLE IF NOT EXISTS game_backups (
    id uuid DEFAULT gen_random_uuid() PRIMARY KEY,
    server_id uuid NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    file_path text NOT NULL,
    size_bytes bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    backup_type character varying(16) DEFAULT 'auto'::character varying NOT NULL,
    CONSTRAINT chk_game_backups_type CHECK (((backup_type)::text = ANY ((ARRAY['auto'::character varying, 'manual'::character varying])::text[])))
);

CREATE INDEX IF NOT EXISTS idx_game_backups_server ON game_backups USING btree (server_id, created_at DESC);

-- ── Seeds (templates de jeux + definition du bot game-bot) ──
INSERT INTO game_templates VALUES ('6f83c629-26ac-4a06-80f0-0b1b99209d97', 'minecraft-vanilla', 'Minecraft Java', 'Serveur Minecraft Java vanilla. Mods/plugins ajoutables ulterieurement.', 'itzg/minecraft-server:latest', 'Survie', '⛏️', '5cb85c', 25565, 2048, 1024, 8192, '{"EULA": "TRUE", "MODE": "survival", "MOTD": "Serveur Minecraft Sentinel", "TYPE": "VANILLA", "VERSION": "LATEST", "DIFFICULTY": "normal", "WHITE_LIST": "false", "ENABLE_RCON": "true", "MAX_PLAYERS": "20", "ONLINE_MODE": "true", "ENABLE_COMMAND_BLOCK": "false"}', '[{"key": "MOTD", "type": "text", "label": "Message d''accueil", "default": "Serveur Minecraft Sentinel", "max_length": 59}, {"key": "MAX_PLAYERS", "max": 200, "min": 1, "type": "number", "label": "Joueurs max", "default": 20}, {"key": "DIFFICULTY", "type": "enum", "label": "Difficulte", "default": "normal", "options": ["peaceful", "easy", "normal", "hard"]}, {"key": "MODE", "type": "enum", "label": "Mode de jeu", "default": "survival", "options": ["survival", "creative", "adventure", "spectator"]}, {"key": "VERSION", "type": "text", "label": "Version Minecraft", "default": "LATEST"}, {"key": "ONLINE_MODE", "type": "boolean", "label": "Mode en ligne (verif Mojang)", "default": "true"}, {"key": "WHITE_LIST", "type": "boolean", "label": "Whitelist activee", "default": "false"}, {"key": "ENABLE_COMMAND_BLOCK", "type": "boolean", "label": "Activer command blocks", "default": "false"}, {"key": "PVP", "type": "boolean", "label": "PvP", "default": "true"}, {"key": "ALLOW_NETHER", "type": "boolean", "label": "Autoriser le Nether", "default": "true"}, {"key": "ANNOUNCE_PLAYER_ACHIEVEMENTS", "type": "boolean", "label": "Annoncer les succes", "default": "true"}, {"key": "SPAWN_ANIMALS", "type": "boolean", "label": "Spawn animaux", "default": "true"}, {"key": "SPAWN_MONSTERS", "type": "boolean", "label": "Spawn monstres", "default": "true"}, {"key": "SPAWN_NPCS", "type": "boolean", "label": "Spawn villageois", "default": "true"}, {"key": "VIEW_DISTANCE", "max": 32, "min": 3, "type": "number", "label": "Distance de vue (chunks)", "default": 10}, {"key": "SIMULATION_DISTANCE", "max": 32, "min": 3, "type": "number", "label": "Distance de simulation (chunks)", "default": 10}]', true, false, 7, NULL, '2026-07-28 14:21:50.20932+00', '2026-07-28 14:21:50.20932+00', 'tcp', NULL, '/data', false, '[]', NULL) ON CONFLICT (slug) DO NOTHING;
INSERT INTO game_templates VALUES ('e044c671-0e19-4c81-8563-4b66b5bab9d8', 'valheim', 'Valheim', 'Survie viking cooperative jusqu''a 10 joueurs. Ports UDP 2456-2458.', 'lloesche/valheim-server:latest', 'Survie', '🪓', 'd4a017', 2456, 3072, 1024, 8192, '{"WORLD_NAME": "Sentinel", "SERVER_NAME": "Sentinel Valheim", "SERVER_PASS": "secret", "SERVER_PUBLIC": "true"}', '[{"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur", "default": "Sentinel Valheim", "max_length": 64}, {"key": "WORLD_NAME", "type": "text", "label": "Nom du monde", "default": "Sentinel", "max_length": 32}, {"key": "SERVER_PASS", "type": "text", "label": "Mot de passe (min 5 chars)", "default": "secret"}, {"key": "SERVER_PUBLIC", "type": "boolean", "label": "Serveur public (visible dans la liste)", "default": "true"}, {"key": "BACKUPS", "type": "boolean", "label": "Sauvegardes auto", "default": "true"}, {"key": "BACKUPS_INTERVAL", "max": 86400, "min": 600, "type": "number", "label": "Interval backup (secondes)", "default": 7200}]', false, false, 7, NULL, '2026-07-28 14:21:50.236391+00', '2026-07-28 14:21:50.254912+00', 'udp', 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/892970/header.jpg', '/config', true, '[]', NULL) ON CONFLICT (slug) DO NOTHING;
INSERT INTO game_templates VALUES ('b57da7a8-7e3f-4488-bd58-2ae2c0853f13', 'factorio', 'Factorio', 'Automatisation et logistique industrielle. Multijoueur cooperatif.', 'factoriotools/factorio:stable', 'Gestion', '⚙️', 'f39c12', 34197, 2048, 1024, 8192, '{"SAVE_NAME": "default", "LOAD_LATEST_SAVE": "true", "GENERATE_NEW_SAVE": "true", "UPDATE_MODS_ON_START": "false"}', '[{"key": "SAVE_NAME", "type": "text", "label": "Nom de la sauvegarde", "default": "default"}, {"key": "GENERATE_NEW_SAVE", "type": "boolean", "label": "Generer une nouvelle save si absente", "default": "true"}, {"key": "LOAD_LATEST_SAVE", "type": "boolean", "label": "Charger la derniere save automatiquement", "default": "true"}, {"key": "UPDATE_MODS_ON_START", "type": "boolean", "label": "Update mods au demarrage", "default": "false"}]', false, true, 7, NULL, '2026-07-28 14:21:50.236391+00', '2026-07-28 14:21:50.254912+00', 'udp', 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/427520/header.jpg', '/factorio', true, '[]', NULL) ON CONFLICT (slug) DO NOTHING;
INSERT INTO game_templates VALUES ('320afa9c-f0a3-4d9c-91cc-07b8558560b1', 'palworld', 'Palworld', 'Survie creatures, jusqu''a 32 joueurs. Tres gourmand en RAM (8 Go conseilles).', 'thijsvanloef/palworld-server-docker:latest', 'Survie', '🐾', '7d5fff', 8211, 8192, 4096, 16384, '{"PLAYERS": "16", "PUBLIC_PORT": "8211", "SERVER_NAME": "Sentinel Palworld", "ADMIN_PASSWORD": "admin", "MULTITHREADING": "true", "SERVER_DESCRIPTION": "Serveur Palworld Sentinel"}', '[{"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur", "default": "Sentinel Palworld"}, {"key": "SERVER_DESCRIPTION", "type": "text", "label": "Description", "default": "Serveur Palworld Sentinel"}, {"key": "ADMIN_PASSWORD", "type": "text", "label": "Mot de passe admin", "default": "admin"}, {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe serveur (vide = libre)", "default": ""}, {"key": "PLAYERS", "max": 32, "min": 1, "type": "number", "label": "Joueurs max", "default": 16}, {"key": "MULTITHREADING", "type": "boolean", "label": "Multithreading", "default": "true"}, {"key": "DEATH_PENALTY", "type": "enum", "label": "Penalite de mort", "default": "All", "options": ["None", "Item", "ItemAndEquipment", "All"]}]', false, false, 7, NULL, '2026-07-28 14:21:50.236391+00', '2026-07-28 14:21:50.254912+00', 'udp', 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/1623730/header.jpg', '/palworld', false, '[]', NULL) ON CONFLICT (slug) DO NOTHING;
INSERT INTO game_templates VALUES ('058b33fc-f1b5-4c99-9ee9-7c4295f86be3', 'ark', 'ARK: Survival Evolved', 'Survie dinosaures, 8-16 Go RAM recommandes. Image hermsi/ark-server.', 'hermsi/ark-server:latest', 'Survie', '🦖', '3a7ca5', 7777, 8192, 4096, 16384, '{"SERVER_MAP": "TheIsland", "MAX_PLAYERS": "20", "SESSION_NAME": "Sentinel ARK", "ADMIN_PASSWORD": "admin", "SERVER_PASSWORD": ""}', '[{"key": "SESSION_NAME", "type": "text", "label": "Nom de session", "default": "Sentinel ARK"}, {"key": "SERVER_MAP", "type": "enum", "label": "Carte", "default": "TheIsland", "options": ["TheIsland", "TheCenter", "ScorchedEarth_P", "Aberration_P", "Extinction", "Genesis", "Gen2", "LostIsland", "Fjordur"]}, {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe (vide = libre)", "default": ""}, {"key": "ADMIN_PASSWORD", "type": "text", "label": "Mot de passe admin RCON", "default": "admin"}, {"key": "MAX_PLAYERS", "max": 70, "min": 1, "type": "number", "label": "Joueurs max", "default": 20}, {"key": "DIFFICULTY_OFFSET", "max": 1, "min": 0, "type": "number", "label": "Difficulty offset", "default": 1}, {"key": "TAMING_SPEED", "max": 100, "min": 1, "type": "number", "label": "Vitesse domestication (multiplicateur)", "default": 1}, {"key": "XP_MULTIPLIER", "max": 100, "min": 1, "type": "number", "label": "XP multiplier", "default": 1}]', false, true, 7, NULL, '2026-07-28 14:21:50.261392+00', '2026-07-28 14:21:50.261392+00', 'udp', 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/346110/header.jpg', '/ark', true, '[]', NULL) ON CONFLICT (slug) DO NOTHING;
INSERT INTO game_templates VALUES ('41b88414-0737-4ec2-9020-cca74b7f0ea1', '7dtd', '7 Days to Die', 'Survie zombies cooperative, 4-8 Go RAM. Image vinanrra/7dtd-server.', 'vinanrra/7dtd-server:latest', 'Survie', '🧟', 'cd412b', 26900, 4096, 2048, 8192, '{"GAME_NAME": "Sentinel", "MAX_PLAYERS": "8", "SERVER_NAME": "Sentinel 7DTD", "GAME_DIFFICULTY": "2", "SERVER_PASSWORD": ""}', '[{"key": "SERVER_NAME", "type": "text", "label": "Nom du serveur", "default": "Sentinel 7DTD"}, {"key": "SERVER_PASSWORD", "type": "text", "label": "Mot de passe (vide = libre)", "default": ""}, {"key": "MAX_PLAYERS", "max": 16, "min": 1, "type": "number", "label": "Joueurs max", "default": 8}, {"key": "GAME_DIFFICULTY", "type": "enum", "label": "Difficulte (0=Scavenger, 5=Insane)", "default": "2", "options": ["0", "1", "2", "3", "4", "5"]}, {"key": "GAME_NAME", "type": "text", "label": "Nom de la partie / monde", "default": "Sentinel"}, {"key": "WORLD_GEN_SEED", "type": "text", "label": "Seed monde (vide = aleatoire)", "default": ""}, {"key": "DAY_NIGHT_LENGTH", "max": 240, "min": 10, "type": "number", "label": "Duree jour/nuit (minutes)", "default": 60}, {"key": "ZOMBIES_RUN", "type": "enum", "label": "Zombies courent (jour)", "default": "0", "options": ["0", "1", "2"]}]', false, true, 7, NULL, '2026-07-28 14:21:50.261392+00', '2026-07-28 14:21:50.261392+00', 'udp', 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/251570/header.jpg', '/home/sdtdserver/.local/share/7DaysToDie', true, '[]', NULL) ON CONFLICT (slug) DO NOTHING;
INSERT INTO game_templates VALUES ('eecb3ccb-fba4-4480-b539-ff5a5cbf2f54', 'terraria', 'Terraria', 'Bac a sable 2D, exploration et boss. Image ryshe (tshock + RCON + plugins).', 'ryshe/terraria:tshock-1.4.5.6-6.1.0', 'Aventure', '🌳', '46b1c9', 7777, 1024, 512, 4096, '{"MOTD": "Welcome to Sentinel Terraria!", "LOGPATH": "/tshock/logs", "PASSWORD": "", "CONFIGPATH": "/tshock", "DIFFICULTY": "0", "WORLD_NAME": "Sentinel", "WORLD_SIZE": "2", "MAX_PLAYERS": "8", "WORLD_FILENAME": "world1.wld"}', '[{"key": "WORLD_NAME", "type": "text", "label": "Nom du monde", "default": "Sentinel"}, {"key": "WORLD_SIZE", "type": "enum", "label": "Taille du monde", "default": "2", "options": ["1", "2", "3"]}, {"key": "DIFFICULTY", "type": "enum", "label": "Difficulte", "default": "0", "options": ["0", "1", "2", "3"]}, {"key": "MAX_PLAYERS", "max": 16, "min": 1, "type": "number", "label": "Joueurs max", "default": 8}, {"key": "MOTD", "type": "text", "label": "Message d''accueil", "default": "Welcome to Sentinel Terraria!"}, {"key": "PASSWORD", "type": "text", "label": "Mot de passe (vide = libre)", "default": ""}]', false, false, 7, NULL, '2026-07-28 14:21:50.236391+00', '2026-07-28 14:21:50.291071+00', 'tcp', 'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/105600/header.jpg', '/root/.local/share/Terraria/Worlds', true, '[{"path": "/tshock/config.json", "content": "{\n  \"Settings\": {\n    \"StorageType\": \"sqlite\",\n    \"ServerPort\": 7777,\n    \"MaxSlots\": {{MAX_PLAYERS}},\n    \"ServerPassword\": \"{{PASSWORD}}\",\n    \"ServerName\": \"{{WORLD_NAME}}\",\n    \"AutoSave\": true\n  }\n}\n"}]', '["-autocreate", "{{WORLD_SIZE}}", "-worldname", "{{WORLD_NAME}}", "-difficulty", "{{DIFFICULTY}}", "-port", "7777", "-maxplayers", "{{MAX_PLAYERS}}", "-motd", "{{MOTD}}"]') ON CONFLICT (slug) DO NOTHING;

INSERT INTO bot_definitions VALUES ('game-bot', 'Games', 'Gestion des jeux mentionnables — les joueurs choisissent leurs jeux et se font ping quand quelqu''un mentionne le jeu.', '[{"key": "enabled", "type": "boolean", "label": "Module actif", "default": "true", "required": false, "description": "Active ou desactive les mini-jeux."}, {"key": "log_channel_id", "type": "channel", "label": "Salon de logs", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Salon ou sont logges les events des mini-jeux (gagnants, scores)."}, {"key": "max_games", "max": 100, "min": 1, "type": "number", "unit": "parties", "label": "Nombre max de jeux par serveur (0 = illimite)", "default": "50", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Nombre max de parties simultanees toutes confondues."}, {"key": "role_color", "type": "text", "label": "Couleur des roles jeux (hex sans #)", "default": "3498db", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "Couleur (hex) des roles attribues aux gagnants. Format : 0xFFA500 ou FFA500."}, {"key": "emoji_host_guild_id", "type": "text", "label": "ID du serveur host des emojis", "required": false, "depends_on": {"key": "enabled", "equals": "true"}, "description": "ID de la guild qui heberge les emojis customs utilises dans les jeux."}]') ON CONFLICT (bot_name) DO NOTHING;
