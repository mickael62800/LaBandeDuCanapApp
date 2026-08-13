-- Migration 164 : Primes collectives (cf. COUPE_AMELIORATIONS 5.3 — vendetta extras).
--
-- Quand un joueur atteint une serie de 5 victoires consecutives, une
-- prime automatique de 1000c apparait sur sa tete. Tout le monde peut
-- contribuer via /contribuer-prime. Le joueur qui le bat empoche le
-- total accumule + un titre "Regicide".

CREATE TABLE IF NOT EXISTS coude_bounties (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     VARCHAR(20) NOT NULL,
    target_id    VARCHAR(20) NOT NULL,
    total_amount BIGINT NOT NULL DEFAULT 0,
    status       VARCHAR(16) NOT NULL DEFAULT 'open',
    opened_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_by   VARCHAR(20),
    claimed_at   TIMESTAMPTZ,
    CHECK (status IN ('open', 'claimed', 'expired'))
);

-- Lookup principal : prime ouverte sur une cible.
CREATE INDEX IF NOT EXISTS idx_coude_bounties_target_open
    ON coude_bounties (guild_id, target_id)
    WHERE status = 'open';

-- Garde-fou : une seule prime ouverte par couple (guild, target).
CREATE UNIQUE INDEX IF NOT EXISTS uniq_coude_bounties_one_open_per_target
    ON coude_bounties (guild_id, target_id)
    WHERE status = 'open';

-- Lookup historique pour /memorial-style listings.
CREATE INDEX IF NOT EXISTS idx_coude_bounties_status_claim
    ON coude_bounties (guild_id, claimed_at DESC NULLS LAST);

-- Optionnel : log des contributions individuelles (pour traçabilite +
-- futur affichage "qui a mise sur la tete de X").
CREATE TABLE IF NOT EXISTS coude_bounty_contributions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bounty_id     UUID NOT NULL REFERENCES coude_bounties(id) ON DELETE CASCADE,
    contributor_id VARCHAR(20) NOT NULL,
    contributor_name VARCHAR(100) NOT NULL,
    amount        BIGINT NOT NULL,
    contributed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_coude_bounty_contributions_bounty
    ON coude_bounty_contributions (bounty_id, contributed_at DESC);
