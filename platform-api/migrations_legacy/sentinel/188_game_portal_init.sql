-- ============================================================================
-- Game Portal — schema initial
-- ============================================================================
-- Catalogue de templates de jeux (Docker images whiteliste'es), instances
-- (game_servers), configs surchargees par instance, sessions joueurs,
-- audit log et backups.
--
-- Architecture hexagonale : ces tables sont la persistance des entites
-- domain/game/*. Aucune logique applicative ici, uniquement le schema +
-- contraintes d'integrite + seed du premier template (Minecraft Vanilla).
--
-- Securite :
--  - Le catalogue (`game_templates`) est la seule source d'images Docker
--    autorisees. Toute creation d'instance verifie que le template_id est
--    valide. Pas de cmd custom ni d'image arbitraire passee depuis l'API.
--  - Les volumes sont nommes (PG : volume_name), pas de bind-mount host.
--  - Les ports sont alloues dans un range configurable cote bot_definitions.

-- ── 1. Templates de jeux ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS game_templates (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Identifiant stable cote code (ex: "minecraft-vanilla"), unique.
    slug                VARCHAR(64) NOT NULL UNIQUE,
    name                VARCHAR(128) NOT NULL,
    description         TEXT,
    -- Image Docker (avec tag). Whiteliste'e.
    image               VARCHAR(255) NOT NULL,
    -- Categorie (Survie, FPS, Aventure, ...) pour groupement UI.
    category            VARCHAR(64),
    -- Icone affichee dans le catalogue (emoji ou unicode).
    icon                VARCHAR(16),
    -- Couleur d'accent (hex sans #) pour la card.
    accent_color        VARCHAR(8),
    -- Port interne du container (ex: 25565 pour Minecraft Java).
    container_port      INTEGER NOT NULL,
    -- Memoire par defaut (Mo) si l'admin n'override pas.
    default_memory_mb   INTEGER NOT NULL DEFAULT 2048,
    -- Memoire min/max acceptables (garde-fou).
    min_memory_mb       INTEGER NOT NULL DEFAULT 512,
    max_memory_mb       INTEGER NOT NULL DEFAULT 8192,
    -- Variables d'environnement par defaut (JSON {KEY: value}).
    -- Ces valeurs sont fusionnees avec les overrides de game_server_configs.
    default_env         JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Schema des champs configurables cote UX game-portal (JSON array).
    -- Format : [{ "key": "...", "label": "...", "type": "text|number|enum|boolean", "default": ..., "options": [...] }]
    config_schema       JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- Le template supporte-t-il RCON ? (true pour Minecraft, certains autres)
    supports_rcon       BOOLEAN NOT NULL DEFAULT FALSE,
    -- Le template supporte-t-il les mods/plugins ? (extensible plus tard)
    supports_mods       BOOLEAN NOT NULL DEFAULT FALSE,
    -- Idle shutdown par defaut (en jours, 0 = desactive).
    idle_shutdown_days  INTEGER NOT NULL DEFAULT 7,
    -- Soft delete pour historique
    deleted_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_game_templates_port CHECK (container_port BETWEEN 1 AND 65535),
    CONSTRAINT chk_game_templates_memory CHECK (
        default_memory_mb >= min_memory_mb
        AND default_memory_mb <= max_memory_mb
        AND min_memory_mb >= 256
        AND max_memory_mb <= 32768
    )
);

CREATE INDEX IF NOT EXISTS idx_game_templates_slug ON game_templates(slug)
    WHERE deleted_at IS NULL;

-- ── 2. Statut des serveurs (enum-style sur VARCHAR + CHECK) ───────────
-- Statuts possibles :
--   created  : ligne creee, container pas encore lance.
--   starting : docker start envoye, attente health.
--   running  : container en cours, repond aux health checks.
--   stopping : docker stop envoye, attente fin.
--   stopped  : container arrete.
--   error    : crash repete ou erreur de boot.
--   deleted  : soft-deleted (volume + container supprimes).

-- ── 3. Instances de serveurs ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS game_servers (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id                VARCHAR(20) NOT NULL,
    template_id             UUID NOT NULL REFERENCES game_templates(id),
    -- Nom donne par l'admin (ex: "Survie-Amis").
    name                    VARCHAR(64) NOT NULL,
    status                  VARCHAR(20) NOT NULL DEFAULT 'created',
    -- Container Docker associe (null tant que non cree). Nom genere = "sentinel-game-{id}".
    container_id            VARCHAR(64),
    container_name          VARCHAR(64),
    -- Port host alloue (mapping host_port:container_port).
    host_port               INTEGER,
    -- Port RCON alloue (si supports_rcon). Range different.
    rcon_port               INTEGER,
    -- Mot de passe RCON (genere aleatoirement, persiste pour reconnexion).
    rcon_password           TEXT,
    -- Volume Docker nomme = "sentinel-game-vol-{id}".
    volume_name             VARCHAR(64),
    -- Memoire allouee (Mo). Sert au quota global de la guild.
    allocated_memory_mb     INTEGER NOT NULL,
    -- Owner Discord (qui a cree le serveur — peut le supprimer en plus de l'Admin/Owner RBAC).
    owner_user_id           VARCHAR(20) NOT NULL,
    -- Idle shutdown specifique a ce serveur (override du template).
    idle_shutdown_days      INTEGER,
    -- Derniere fois qu'un joueur etait connecte (mis a jour par game-portal-worker).
    last_active_at          TIMESTAMPTZ,
    -- Dernier nb de joueurs vu par health-check.
    last_player_count       INTEGER NOT NULL DEFAULT 0,
    -- Derniere erreur eventuelle (visible UX).
    last_error              TEXT,
    -- Timestamps
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at              TIMESTAMPTZ,
    stopped_at              TIMESTAMPTZ,
    deleted_at              TIMESTAMPTZ,
    CONSTRAINT chk_game_servers_status CHECK (status IN (
        'created', 'starting', 'running', 'stopping', 'stopped', 'error', 'deleted'
    )),
    CONSTRAINT chk_game_servers_name CHECK (
        char_length(name) BETWEEN 1 AND 64
        AND name ~ '^[a-zA-Z0-9 _\-]+$'
    ),
    CONSTRAINT chk_game_servers_memory CHECK (allocated_memory_mb >= 256 AND allocated_memory_mb <= 32768),
    CONSTRAINT chk_game_servers_host_port CHECK (host_port IS NULL OR (host_port BETWEEN 1024 AND 65535)),
    CONSTRAINT chk_game_servers_rcon_port CHECK (rcon_port IS NULL OR (rcon_port BETWEEN 1024 AND 65535))
);

-- Unicite des ports alloues (un host_port = un seul serveur actif).
-- Soft-delete preserve l'historique : on filtre via partial unique.
CREATE UNIQUE INDEX IF NOT EXISTS uq_game_servers_host_port
    ON game_servers(host_port)
    WHERE host_port IS NOT NULL AND deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_game_servers_rcon_port
    ON game_servers(rcon_port)
    WHERE rcon_port IS NOT NULL AND deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_game_servers_container_name
    ON game_servers(container_name)
    WHERE container_name IS NOT NULL AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_game_servers_guild
    ON game_servers(guild_id)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_game_servers_status
    ON game_servers(status)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_game_servers_last_active
    ON game_servers(last_active_at)
    WHERE deleted_at IS NULL AND status = 'running';

-- ── 4. Configs surchargees par instance (key/value) ───────────────────
-- Override des default_env et des champs declares dans config_schema du template.
CREATE TABLE IF NOT EXISTS game_server_configs (
    server_id   UUID NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    config_key  VARCHAR(64) NOT NULL,
    config_value TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by  VARCHAR(20),
    PRIMARY KEY (server_id, config_key),
    CONSTRAINT chk_game_server_configs_key CHECK (
        char_length(config_key) BETWEEN 1 AND 64
        AND config_key ~ '^[A-Z][A-Z0-9_]*$'
    )
);

-- ── 5. Sessions joueurs ───────────────────────────────────────────────
-- Tracking des connexions/deconnexions detectees par le worker
-- (RCON list players + diff entre 2 polls).
CREATE TABLE IF NOT EXISTS game_player_sessions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id   UUID NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    -- Pseudo Minecraft / username Steam — pas de FK Discord (les joueurs
    -- ne sont pas forcement sur le serveur Discord).
    player_name VARCHAR(64) NOT NULL,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at     TIMESTAMPTZ,
    duration_seconds INTEGER GENERATED ALWAYS AS (
        CASE WHEN left_at IS NOT NULL
             THEN EXTRACT(EPOCH FROM (left_at - joined_at))::int
             ELSE NULL
        END
    ) STORED
);

CREATE INDEX IF NOT EXISTS idx_game_player_sessions_server
    ON game_player_sessions(server_id, joined_at DESC);

CREATE INDEX IF NOT EXISTS idx_game_player_sessions_active
    ON game_player_sessions(server_id, player_name)
    WHERE left_at IS NULL;

-- ── 6. Audit log des actions ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS game_audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID REFERENCES game_servers(id) ON DELETE SET NULL,
    guild_id        VARCHAR(20) NOT NULL,
    actor_user_id   VARCHAR(20),
    -- Actions : create, start, stop, restart, delete, config_update, command_rcon, idle_shutdown,
    --           crash_detected, auto_restart, backup_create, backup_restore.
    action          VARCHAR(32) NOT NULL,
    details         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_game_audit_log_server
    ON game_audit_log(server_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_game_audit_log_guild
    ON game_audit_log(guild_id, created_at DESC);

-- ── 7. Backups (phase 2 mais schema prevu maintenant) ─────────────────
CREATE TABLE IF NOT EXISTS game_backups (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id       UUID NOT NULL REFERENCES game_servers(id) ON DELETE CASCADE,
    file_path       TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Type de backup : auto (cron worker) ou manual (action UX).
    backup_type     VARCHAR(16) NOT NULL DEFAULT 'auto',
    CONSTRAINT chk_game_backups_type CHECK (backup_type IN ('auto', 'manual'))
);

CREATE INDEX IF NOT EXISTS idx_game_backups_server
    ON game_backups(server_id, created_at DESC);

-- ── 8. Seed du premier template : Minecraft Java Vanilla ──────────────
INSERT INTO game_templates (
    slug, name, description, image, category, icon, accent_color,
    container_port, default_memory_mb, min_memory_mb, max_memory_mb,
    default_env, config_schema, supports_rcon, supports_mods, idle_shutdown_days
) VALUES (
    'minecraft-vanilla',
    'Minecraft Java',
    'Serveur Minecraft Java vanilla. Mods/plugins ajoutables ulterieurement.',
    'itzg/minecraft-server:latest',
    'Survie',
    '⛏️',
    '5cb85c',
    25565,
    2048,
    1024,
    8192,
    -- Variables d'environnement par defaut (image itzg/minecraft-server).
    -- EULA accepte automatiquement (sinon le container ne demarre pas).
    '{
        "EULA": "TRUE",
        "TYPE": "VANILLA",
        "VERSION": "LATEST",
        "ENABLE_RCON": "true",
        "MAX_PLAYERS": "20",
        "MOTD": "Serveur Minecraft Sentinel",
        "DIFFICULTY": "normal",
        "MODE": "survival",
        "ONLINE_MODE": "true",
        "WHITE_LIST": "false",
        "ENABLE_COMMAND_BLOCK": "false"
    }'::jsonb,
    -- Schema config UX (game-portal exposera ces champs comme inputs).
    '[
        {"key": "MOTD", "label": "Message d''accueil", "type": "text", "default": "Serveur Minecraft Sentinel", "max_length": 59},
        {"key": "MAX_PLAYERS", "label": "Joueurs max", "type": "number", "default": 20, "min": 1, "max": 200},
        {"key": "DIFFICULTY", "label": "Difficulte", "type": "enum", "default": "normal", "options": ["peaceful", "easy", "normal", "hard"]},
        {"key": "MODE", "label": "Mode de jeu", "type": "enum", "default": "survival", "options": ["survival", "creative", "adventure", "spectator"]},
        {"key": "VERSION", "label": "Version Minecraft", "type": "text", "default": "LATEST"},
        {"key": "ONLINE_MODE", "label": "Mode en ligne (verif Mojang)", "type": "boolean", "default": "true"},
        {"key": "WHITE_LIST", "label": "Whitelist activee", "type": "boolean", "default": "false"},
        {"key": "ENABLE_COMMAND_BLOCK", "label": "Activer command blocks", "type": "boolean", "default": "false"},
        {"key": "PVP", "label": "PvP", "type": "boolean", "default": "true"},
        {"key": "ALLOW_NETHER", "label": "Autoriser le Nether", "type": "boolean", "default": "true"},
        {"key": "ANNOUNCE_PLAYER_ACHIEVEMENTS", "label": "Annoncer les succes", "type": "boolean", "default": "true"},
        {"key": "SPAWN_ANIMALS", "label": "Spawn animaux", "type": "boolean", "default": "true"},
        {"key": "SPAWN_MONSTERS", "label": "Spawn monstres", "type": "boolean", "default": "true"},
        {"key": "SPAWN_NPCS", "label": "Spawn villageois", "type": "boolean", "default": "true"},
        {"key": "VIEW_DISTANCE", "label": "Distance de vue (chunks)", "type": "number", "default": 10, "min": 3, "max": 32},
        {"key": "SIMULATION_DISTANCE", "label": "Distance de simulation (chunks)", "type": "number", "default": 10, "min": 3, "max": 32}
    ]'::jsonb,
    TRUE,
    FALSE,  -- mods/plugins extensibles plus tard via TYPE=PAPER/FORGE/FABRIC
    7
)
ON CONFLICT (slug) DO UPDATE SET
    name = EXCLUDED.name,
    description = EXCLUDED.description,
    image = EXCLUDED.image,
    config_schema = EXCLUDED.config_schema,
    default_env = EXCLUDED.default_env,
    updated_at = NOW();
