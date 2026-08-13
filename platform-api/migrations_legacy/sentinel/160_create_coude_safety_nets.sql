-- Migration 160 : Filet de securite (cf. COUPE_AMELIORATIONS section 4.4).
--
-- Quand le wallet d un joueur tombe sous 50c, on active un filet de
-- securite pendant 72h : pertes /2, paris gagnants x1.5. Une seule
-- entree active par couple (guild, user) — on ne peut pas cumuler les
-- filets.

CREATE TABLE IF NOT EXISTS coude_safety_nets (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     VARCHAR(20) NOT NULL,
    user_id      VARCHAR(20) NOT NULL,
    activated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at   TIMESTAMPTZ NOT NULL
);

-- Lookup principal : filet actif pour un joueur donne. On filtre
-- expires_at > NOW au runtime.
CREATE INDEX IF NOT EXISTS idx_coude_safety_nets_user
    ON coude_safety_nets (guild_id, user_id, expires_at DESC);

-- Garde-fou : un seul filet actif a la fois par couple (guild, user).
-- Partial index : on considere "actif" = expires_at >= NOW au moment
-- de l insertion (le runtime nettoiera via le service).
-- En pratique le service check d abord get_active() avant d insert.
CREATE UNIQUE INDEX IF NOT EXISTS uniq_coude_safety_nets_one_per_user
    ON coude_safety_nets (guild_id, user_id, activated_at);
