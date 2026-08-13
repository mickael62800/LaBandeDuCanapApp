-- ============================================================================
-- Game Portal — ajout ARK et 7 Days to Die
-- ============================================================================
-- Note importante :
--   - ARK et 7DTD : images Docker Linux maintenues, OK.
--   - Astroneer : serveur dedie WINDOWS-only (pas d'image Docker Linux
--     stable). Non ajoute. L'admin peut le contourner via Wine + image
--     custom mais c'est hors scope.
--   - Farming Simulator (22/25) : meme cas, serveur Windows-only.
--     Pas d'image Docker Linux fonctionnelle. Non ajoute.

-- ── ARK: Survival Evolved (UDP) ──────────────────────────────────────
-- Image hermsi/ark-server : tres populaire, ~8 Go RAM minimum.
-- Le port principal est 7777/udp. Les ports 27015/udp (query) et
-- 7778/udp (raw socket) ne sont pas exposes par notre allocator
-- single-port mais le jeu fonctionne quand meme avec 7777 seul.
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, volume_path, run_as_root,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days,
    cover_image_url
) VALUES (
    'ark',
    'ARK: Survival Evolved',
    'Survie dinosaures, 8-16 Go RAM recommandes. Image hermsi/ark-server.',
    'hermsi/ark-server:latest',
    'Survie',
    '🦖',
    '3a7ca5',
    7777,
    'udp',
    '/ark',
    TRUE,
    8192,
    4096,
    16384,
    '{
        "SESSION_NAME": "Sentinel ARK",
        "SERVER_MAP": "TheIsland",
        "SERVER_PASSWORD": "",
        "ADMIN_PASSWORD": "admin",
        "MAX_PLAYERS": "20"
    }'::jsonb,
    '[
        {"key": "SESSION_NAME", "label": "Nom de session", "type": "text", "default": "Sentinel ARK"},
        {"key": "SERVER_MAP", "label": "Carte", "type": "enum", "default": "TheIsland", "options": ["TheIsland", "TheCenter", "ScorchedEarth_P", "Aberration_P", "Extinction", "Genesis", "Gen2", "LostIsland", "Fjordur"]},
        {"key": "SERVER_PASSWORD", "label": "Mot de passe (vide = libre)", "type": "text", "default": ""},
        {"key": "ADMIN_PASSWORD", "label": "Mot de passe admin RCON", "type": "text", "default": "admin"},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 20, "min": 1, "max": 70},
        {"key": "DIFFICULTY_OFFSET", "label": "Difficulty offset", "type": "number", "default": 1, "min": 0, "max": 1},
        {"key": "TAMING_SPEED", "label": "Vitesse domestication (multiplicateur)", "type": "number", "default": 1, "min": 1, "max": 100},
        {"key": "XP_MULTIPLIER", "label": "XP multiplier", "type": "number", "default": 1, "min": 1, "max": 100}
    ]'::jsonb,
    FALSE,
    TRUE,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/346110/header.jpg'
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    volume_path = EXCLUDED.volume_path,
    run_as_root = EXCLUDED.run_as_root,
    cover_image_url = EXCLUDED.cover_image_url,
    updated_at = NOW();

-- ── 7 Days to Die (UDP) ──────────────────────────────────────────────
-- Image vinanrra/7dtd-server : maintained, port 26900/udp.
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, port_protocol, volume_path, run_as_root,
    default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days,
    cover_image_url
) VALUES (
    '7dtd',
    '7 Days to Die',
    'Survie zombies cooperative, 4-8 Go RAM. Image vinanrra/7dtd-server.',
    'vinanrra/7dtd-server:latest',
    'Survie',
    '🧟',
    'cd412b',
    26900,
    'udp',
    '/home/sdtdserver/.local/share/7DaysToDie',
    TRUE,
    4096,
    2048,
    8192,
    '{
        "SERVER_NAME": "Sentinel 7DTD",
        "SERVER_PASSWORD": "",
        "MAX_PLAYERS": "8",
        "GAME_DIFFICULTY": "2",
        "GAME_NAME": "Sentinel"
    }'::jsonb,
    '[
        {"key": "SERVER_NAME", "label": "Nom du serveur", "type": "text", "default": "Sentinel 7DTD"},
        {"key": "SERVER_PASSWORD", "label": "Mot de passe (vide = libre)", "type": "text", "default": ""},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 8, "min": 1, "max": 16},
        {"key": "GAME_DIFFICULTY", "label": "Difficulte (0=Scavenger, 5=Insane)", "type": "enum", "default": "2", "options": ["0", "1", "2", "3", "4", "5"]},
        {"key": "GAME_NAME", "label": "Nom de la partie / monde", "type": "text", "default": "Sentinel"},
        {"key": "WORLD_GEN_SEED", "label": "Seed monde (vide = aleatoire)", "type": "text", "default": ""},
        {"key": "DAY_NIGHT_LENGTH", "label": "Duree jour/nuit (minutes)", "type": "number", "default": 60, "min": 10, "max": 240},
        {"key": "ZOMBIES_RUN", "label": "Zombies courent (jour)", "type": "enum", "default": "0", "options": ["0", "1", "2"]}
    ]'::jsonb,
    FALSE,
    TRUE,
    7,
    'https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/251570/header.jpg'
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    volume_path = EXCLUDED.volume_path,
    run_as_root = EXCLUDED.run_as_root,
    cover_image_url = EXCLUDED.cover_image_url,
    updated_at = NOW();

-- ── allowed_templates default etendu ─────────────────────────────────
-- (Les guilds sans override voient automatiquement les nouveaux jeux.)
-- Note : on touche uniquement le `default` du schema, pas les overrides
-- guild existants (UPDATE bot_guild_config est explicite cote user).
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
                    '"minecraft-vanilla,valheim,terraria,factorio,palworld,ark,7dtd"'::jsonb
                )
                ELSE entry
            END
        )
        FROM jsonb_array_elements(config_schema::jsonb) AS entry
    )
)
WHERE bot_name = 'game-portal';
