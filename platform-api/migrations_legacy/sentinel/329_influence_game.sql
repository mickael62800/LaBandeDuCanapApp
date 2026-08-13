-- Jeu « Influence » — Phase 1 (MVP) : identite citoyenne (5 capitaux),
-- organisations + adhesion, votes simples, et archives (append-only).
-- Voir docs/Nouveau jeux/ARCHITECTURE.md. Toutes les tables sont multi-serveur
-- (guild_id TEXT NOT NULL) et prefixees `influence_`. Migration idempotente.

-- ── Citoyens : racine, 5 capitaux stockes en entier (exposes en paliers) ──
CREATE TABLE IF NOT EXISTS influence_citizens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT   NOT NULL,
    user_id      TEXT   NOT NULL,
    username     TEXT   NOT NULL DEFAULT '',
    influence    BIGINT NOT NULL DEFAULT 0,
    money        BIGINT NOT NULL DEFAULT 0,
    reputation   BIGINT NOT NULL DEFAULT 0,
    information  BIGINT NOT NULL DEFAULT 0,
    network      BIGINT NOT NULL DEFAULT 0,
    joined_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, user_id)
);

-- ── Organisations : Entreprise|Parti|Media|Syndicat|Secrete ──
CREATE TABLE IF NOT EXISTS influence_organizations (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT   NOT NULL,
    kind         TEXT   NOT NULL,               -- entreprise|parti|media|syndicat|secrete
    name         TEXT   NOT NULL,
    motto        TEXT   NOT NULL DEFAULT '',
    treasury     BIGINT NOT NULL DEFAULT 0,
    reputation   BIGINT NOT NULL DEFAULT 0,
    influence    BIGINT NOT NULL DEFAULT 0,
    founder_id   UUID   NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    dissolved_at TIMESTAMPTZ,
    UNIQUE (guild_id, name)
);
CREATE INDEX IF NOT EXISTS idx_influence_orgs_guild
    ON influence_organizations (guild_id) WHERE dissolved_at IS NULL;

-- ── Adhesions : hierarchie Fondateur..Recrue ──
CREATE TABLE IF NOT EXISTS influence_org_members (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES influence_organizations(id) ON DELETE CASCADE,
    citizen_id  UUID NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    role        TEXT NOT NULL DEFAULT 'recrue',  -- fondateur|dirigeant|responsable|membre|recrue
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (org_id, citizen_id)
);
CREATE INDEX IF NOT EXISTS idx_influence_org_members_citizen
    ON influence_org_members (citizen_id);

-- ── Motions : sujet d'un vote binaire simple (au sein d'une org) ──
CREATE TABLE IF NOT EXISTS influence_motions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    org_id      UUID REFERENCES influence_organizations(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'ouverte',  -- ouverte|adoptee|rejetee
    created_by  UUID NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closes_at   TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_influence_motions_close
    ON influence_motions (closes_at) WHERE status = 'ouverte';

-- ── Votes : un bulletin par citoyen et par motion ──
CREATE TABLE IF NOT EXISTS influence_votes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subject_type TEXT NOT NULL DEFAULT 'motion',  -- motion|law|election (extensible)
    subject_id   UUID NOT NULL,
    voter_id     UUID NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    choice       TEXT NOT NULL,                   -- pour|contre|abstention
    secret       BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (subject_id, voter_id)
);

-- ── Archives : memoire immuable du serveur (append-only, jamais purge) ──
CREATE TABLE IF NOT EXISTS influence_archives (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    payload     JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_influence_archives_guild
    ON influence_archives (guild_id, occurred_at DESC);

-- ── Definition du bot + config web (page Composants) ──
INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'influence-bot',
    'Influence',
    'Jeu de strategie sociale et politique : les citoyens accumulent 5 capitaux (Influence, Argent, Reputation, Information, Reseau), fondent des organisations et votent.',
    '[
        {"key": "enabled", "label": "Module actif", "type": "boolean", "required": false, "default": "false", "description": "Active le jeu Influence sur ce serveur."},
        {"key": "influence_start_money", "label": "Argent de depart d un citoyen", "type": "number", "required": false, "default": "1000", "description": "Capital Argent initial a la premiere activite."},
        {"key": "influence_org_creation_cost", "label": "Cout de creation d une organisation", "type": "number", "required": false, "default": "1000", "description": "Argent preleve au fondateur pour creer une organisation."},
        {"key": "influence_org_max_per_citizen", "label": "Organisations max fondees par citoyen", "type": "number", "required": false, "default": "3", "description": "Nombre d organisations qu un meme citoyen peut fonder."},
        {"key": "influence_mandate_days", "label": "Duree d un mandat (jours)", "type": "number", "required": false, "default": "14", "description": "Duree par defaut d un mandat politique (phases ulterieures)."},
        {"key": "influence_law_debate_hours", "label": "Duree du debat d une loi (h)", "type": "number", "required": false, "default": "48", "description": "Duree du debat avant vote d une loi (phases ulterieures)."}
    ]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE
    SET display_name = EXCLUDED.display_name,
        description  = EXCLUDED.description,
        config_schema = EXCLUDED.config_schema;
