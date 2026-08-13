-- ============================================================================
-- Game Portal — support port_protocol (TCP/UDP) + 4 nouveaux templates
-- ============================================================================

-- 1. Ajout de la colonne port_protocol (TCP par defaut pour retro-compat).
ALTER TABLE game_templates
    ADD COLUMN IF NOT EXISTS port_protocol VARCHAR(8) NOT NULL DEFAULT 'tcp';

ALTER TABLE game_templates
    DROP CONSTRAINT IF EXISTS chk_game_templates_protocol;
ALTER TABLE game_templates
    ADD CONSTRAINT chk_game_templates_protocol
    CHECK (port_protocol IN ('tcp', 'udp'));

-- 2. Seed des nouveaux templates.

-- ── Valheim (UDP) ──────────────────────────────────────────────────────
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days
) VALUES (
    'valheim',
    'Valheim',
    'Survie viking cooperative jusqu''a 10 joueurs. Ports UDP 2456-2458.',
    'lloesche/valheim-server:latest',
    'Survie',
    '🪓',
    'd4a017',
    2456,
    'udp',
    3072,
    1024,
    8192,
    '{
        "SERVER_NAME": "Sentinel Valheim",
        "WORLD_NAME": "Sentinel",
        "SERVER_PASS": "secret",
        "SERVER_PUBLIC": "true"
    }'::jsonb,
    '[
        {"key": "SERVER_NAME", "label": "Nom du serveur", "type": "text", "default": "Sentinel Valheim", "max_length": 64},
        {"key": "WORLD_NAME", "label": "Nom du monde", "type": "text", "default": "Sentinel", "max_length": 32},
        {"key": "SERVER_PASS", "label": "Mot de passe (min 5 chars)", "type": "text", "default": "secret"},
        {"key": "SERVER_PUBLIC", "label": "Serveur public (visible dans la liste)", "type": "boolean", "default": "true"},
        {"key": "BACKUPS", "label": "Sauvegardes auto", "type": "boolean", "default": "true"},
        {"key": "BACKUPS_INTERVAL", "label": "Interval backup (secondes)", "type": "number", "default": 7200, "min": 600, "max": 86400}
    ]'::jsonb,
    FALSE,
    FALSE,
    7
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    port_protocol = EXCLUDED.port_protocol,
    updated_at = NOW();

-- ── Terraria (TCP) ─────────────────────────────────────────────────────
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days
) VALUES (
    'terraria',
    'Terraria',
    'Bac a sable 2D, exploration et boss. Jusqu''a 8 joueurs.',
    'ryshe/terraria:latest',
    'Aventure',
    '🌳',
    '46b1c9',
    7777,
    'tcp',
    1024,
    512,
    4096,
    '{
        "WORLD_FILENAME": "world1.wld",
        "WORLD_SIZE": "medium",
        "DIFFICULTY": "normal",
        "MAX_PLAYERS": "8"
    }'::jsonb,
    '[
        {"key": "WORLD_FILENAME", "label": "Nom du fichier monde", "type": "text", "default": "world1.wld"},
        {"key": "WORLD_SIZE", "label": "Taille du monde", "type": "enum", "default": "medium", "options": ["small", "medium", "large"]},
        {"key": "DIFFICULTY", "label": "Difficulte", "type": "enum", "default": "normal", "options": ["normal", "expert", "master", "journey"]},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Welcome to Sentinel Terraria!"},
        {"key": "PASSWORD", "label": "Mot de passe (vide = pas de mdp)", "type": "text", "default": ""}
    ]'::jsonb,
    FALSE,
    FALSE,
    7
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    port_protocol = EXCLUDED.port_protocol,
    updated_at = NOW();

-- ── Factorio (UDP) ────────────────────────────────────────────────────
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days
) VALUES (
    'factorio',
    'Factorio',
    'Automatisation et logistique industrielle. Multijoueur cooperatif.',
    'factoriotools/factorio:stable',
    'Gestion',
    '⚙️',
    'f39c12',
    34197,
    'udp',
    2048,
    1024,
    8192,
    '{
        "SAVE_NAME": "default",
        "GENERATE_NEW_SAVE": "true",
        "LOAD_LATEST_SAVE": "true",
        "UPDATE_MODS_ON_START": "false"
    }'::jsonb,
    '[
        {"key": "SAVE_NAME", "label": "Nom de la sauvegarde", "type": "text", "default": "default"},
        {"key": "GENERATE_NEW_SAVE", "label": "Generer une nouvelle save si absente", "type": "boolean", "default": "true"},
        {"key": "LOAD_LATEST_SAVE", "label": "Charger la derniere save automatiquement", "type": "boolean", "default": "true"},
        {"key": "UPDATE_MODS_ON_START", "label": "Update mods au demarrage", "type": "boolean", "default": "false"}
    ]'::jsonb,
    FALSE,
    TRUE,
    7
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    port_protocol = EXCLUDED.port_protocol,
    updated_at = NOW();

-- ── Palworld (UDP) ────────────────────────────────────────────────────
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days
) VALUES (
    'palworld',
    'Palworld',
    'Survie creatures, jusqu''a 32 joueurs. Tres gourmand en RAM (8 Go conseilles).',
    'thijsvanloef/palworld-server-docker:latest',
    'Survie',
    '🐾',
    '7d5fff',
    8211,
    'udp',
    8192,
    4096,
    16384,
    '{
        "SERVER_NAME": "Sentinel Palworld",
        "SERVER_DESCRIPTION": "Serveur Palworld Sentinel",
        "ADMIN_PASSWORD": "admin",
        "MULTITHREADING": "true",
        "PUBLIC_PORT": "8211",
        "PLAYERS": "16"
    }'::jsonb,
    '[
        {"key": "SERVER_NAME", "label": "Nom du serveur", "type": "text", "default": "Sentinel Palworld"},
        {"key": "SERVER_DESCRIPTION", "label": "Description", "type": "text", "default": "Serveur Palworld Sentinel"},
        {"key": "ADMIN_PASSWORD", "label": "Mot de passe admin", "type": "text", "default": "admin"},
        {"key": "SERVER_PASSWORD", "label": "Mot de passe serveur (vide = libre)", "type": "text", "default": ""},
        {"key": "PLAYERS", "label": "Joueurs max", "type": "number", "default": 16, "min": 1, "max": 32},
        {"key": "MULTITHREADING", "label": "Multithreading", "type": "boolean", "default": "true"},
        {"key": "DEATH_PENALTY", "label": "Penalite de mort", "type": "enum", "default": "All", "options": ["None", "Item", "ItemAndEquipment", "All"]}
    ]'::jsonb,
    FALSE,
    FALSE,
    7
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    port_protocol = EXCLUDED.port_protocol,
    updated_at = NOW();

-- 3. Etend allowed_templates par defaut dans bot_definitions pour que les
--    nouvelles guilds aient acces aux 5 jeux directement. Les guilds
--    existantes gardent leur valeur configuree (pas d'override).
UPDATE bot_definitions
SET config_schema = jsonb_set(
    config_schema::jsonb,
    '{}',
    (
        SELECT jsonb_agg(
            CASE
                WHEN entry->>'key' = 'allowed_templates'
                THEN jsonb_set(
                    entry,
                    '{default}',
                    '"minecraft-vanilla,valheim,terraria,factorio,palworld"'::jsonb
                )
                ELSE entry
            END
        )
        FROM jsonb_array_elements(config_schema::jsonb) AS entry
    )
)
WHERE bot_name = 'game-portal';
