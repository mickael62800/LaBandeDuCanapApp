-- Tables de paris et assurances pour le jeu Coup de Coude

CREATE TABLE IF NOT EXISTS coude_bets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    combat_id   UUID NOT NULL REFERENCES coude_combats(id),
    bettor_id   TEXT NOT NULL,
    bettor_name TEXT NOT NULL,
    backed_id   TEXT NOT NULL,
    amount      BIGINT NOT NULL,
    won         BOOLEAN,
    payout      BIGINT DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_coude_bets_combat ON coude_bets(combat_id);

CREATE TABLE IF NOT EXISTS coude_insurances (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    is_scam     BOOLEAN NOT NULL DEFAULT FALSE,
    active      BOOLEAN NOT NULL DEFAULT TRUE,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_coude_insurances_active ON coude_insurances(guild_id, user_id, active) WHERE active = TRUE;
