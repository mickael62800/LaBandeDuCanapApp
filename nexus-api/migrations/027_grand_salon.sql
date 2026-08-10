-- Le Grand Salon — jeu social de La Bande du Canapé.
-- Tables propres à Nexus : aucune réutilisation des anciennes tables Sentinel.

CREATE TABLE IF NOT EXISTS grand_salon_habitues (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    rayonnement BIGINT NOT NULL DEFAULT 0,
    jetons BIGINT NOT NULL DEFAULT 0 CHECK (jetons >= 0),
    reputation BIGINT NOT NULL DEFAULT 0,
    bons_plans BIGINT NOT NULL DEFAULT 0,
    reseau BIGINT NOT NULL DEFAULT 0,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, user_id)
);

CREATE TABLE IF NOT EXISTS grand_salon_cercles (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('bande', 'club', 'collectif')),
    name TEXT NOT NULL,
    devise TEXT NOT NULL DEFAULT '',
    caisse BIGINT NOT NULL DEFAULT 0 CHECK (caisse >= 0),
    reputation BIGINT NOT NULL DEFAULT 0,
    rayonnement BIGINT NOT NULL DEFAULT 0,
    founder_id UUID NOT NULL REFERENCES grand_salon_habitues(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    dissolved_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS grand_salon_cercles_active_idx
    ON grand_salon_cercles (guild_id) WHERE dissolved_at IS NULL;

CREATE TABLE IF NOT EXISTS grand_salon_cercle_members (
    cercle_id UUID NOT NULL REFERENCES grand_salon_cercles(id) ON DELETE CASCADE,
    habitue_id UUID NOT NULL REFERENCES grand_salon_habitues(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'membre' CHECK (role IN ('fondateur', 'responsable', 'membre')),
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (cercle_id, habitue_id)
);

CREATE TABLE IF NOT EXISTS grand_salon_motions (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    titre TEXT NOT NULL,
    texte TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('en_vote', 'adoptee', 'rejetee')),
    author_id UUID NOT NULL REFERENCES grand_salon_habitues(id),
    closes_at TIMESTAMPTZ NOT NULL,
    soutien_pour BIGINT NOT NULL DEFAULT 0,
    soutien_contre BIGINT NOT NULL DEFAULT 0,
    closed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS grand_salon_motions_due_idx
    ON grand_salon_motions (closes_at) WHERE status = 'en_vote';

CREATE TABLE IF NOT EXISTS grand_salon_votes (
    motion_id UUID NOT NULL REFERENCES grand_salon_motions(id) ON DELETE CASCADE,
    habitue_id UUID NOT NULL REFERENCES grand_salon_habitues(id) ON DELETE CASCADE,
    choice BOOLEAN NOT NULL,
    weight BIGINT NOT NULL DEFAULT 1 CHECK (weight > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (motion_id, habitue_id)
);

CREATE TABLE IF NOT EXISTS grand_salon_daily_claims (
    habitue_id UUID NOT NULL REFERENCES grand_salon_habitues(id) ON DELETE CASCADE,
    claim_date DATE NOT NULL DEFAULT CURRENT_DATE,
    PRIMARY KEY (habitue_id, claim_date)
);

CREATE TABLE IF NOT EXISTS grand_salon_dossiers (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    owner_id UUID NOT NULL REFERENCES grand_salon_habitues(id),
    subject TEXT NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    revealed_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS grand_salon_gazette (
    id UUID PRIMARY KEY,
    guild_id TEXT NOT NULL,
    headline TEXT NOT NULL,
    body TEXT NOT NULL,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS grand_salon_gazette_recent_idx
    ON grand_salon_gazette (guild_id, published_at DESC);

INSERT INTO bot_definitions (bot_name, display_name, description, config_schema)
VALUES (
    'grand-salon',
    'Le Grand Salon',
    'Jeu social de La Bande du Canapé : habitués, cercles, motions, enquêtes et Gazette.',
    '[{"key":"enabled","type":"boolean","label":"Jeu actif","default":"true","required":false},{"key":"starting_jetons","type":"number","label":"Jetons de départ","default":"1000","min":0,"required":false},{"key":"motion_duration_hours","type":"number","label":"Durée des motions","default":"48","min":1,"max":168,"required":false}]'::jsonb
)
ON CONFLICT (bot_name) DO UPDATE SET
    display_name = EXCLUDED.display_name,
    description = EXCLUDED.description,
    config_schema = EXCLUDED.config_schema;
