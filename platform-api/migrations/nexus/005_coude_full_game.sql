-- Fonctionnalites completes de Coup de Coude. Les coins restent uniquement
-- dans nexus_wallets ; ces tables ne stockent que l'etat du jeu.

ALTER TABLE nexus_coude_players
    ADD COLUMN IF NOT EXISTS title VARCHAR(32) NOT NULL DEFAULT 'Debutant',
    ADD COLUMN IF NOT EXISTS class_changed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
CREATE INDEX IF NOT EXISTS idx_nexus_coude_players_level
    ON nexus_coude_players (guild_id, level DESC, xp DESC);

ALTER TABLE nexus_coude_combats
    ADD COLUMN IF NOT EXISTS accepted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS defender_special VARCHAR(64),
    ADD COLUMN IF NOT EXISTS message_id VARCHAR(20);

CREATE TABLE IF NOT EXISTS nexus_coude_bets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id VARCHAR(20) NOT NULL,
    combat_id UUID NOT NULL REFERENCES nexus_coude_combats(id) ON DELETE CASCADE,
    bettor_id VARCHAR(20) NOT NULL,
    bettor_name VARCHAR(100) NOT NULL,
    backed_id VARCHAR(20) NOT NULL,
    amount BIGINT NOT NULL CHECK (amount > 0),
    won BOOLEAN,
    payout BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (combat_id, bettor_id)
);
CREATE INDEX IF NOT EXISTS idx_nexus_coude_bets_combat ON nexus_coude_bets (combat_id);

CREATE TABLE IF NOT EXISTS nexus_coude_primes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id VARCHAR(20) NOT NULL,
    target_id VARCHAR(20) NOT NULL,
    target_name VARCHAR(100) NOT NULL,
    placed_by_id VARCHAR(20) NOT NULL,
    placed_by_name VARCHAR(100) NOT NULL,
    amount BIGINT NOT NULL CHECK (amount > 0),
    claimed BOOLEAN NOT NULL DEFAULT FALSE,
    claimed_by_id VARCHAR(20),
    claimed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_nexus_coude_primes_target
    ON nexus_coude_primes (guild_id, target_id) WHERE claimed = FALSE;

CREATE TABLE IF NOT EXISTS nexus_coude_insurances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id VARCHAR(20) NOT NULL,
    user_id VARCHAR(20) NOT NULL,
    is_scam BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_nexus_coude_insurances_active
    ON nexus_coude_insurances (guild_id, user_id) WHERE active = TRUE;

CREATE TABLE IF NOT EXISTS nexus_coude_cooldowns (
    guild_id VARCHAR(20) NOT NULL,
    user_id VARCHAR(20) NOT NULL,
    action VARCHAR(32) NOT NULL,
    available_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (guild_id, user_id, action)
);

CREATE TABLE IF NOT EXISTS nexus_coude_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id VARCHAR(20) NOT NULL,
    event_type VARCHAR(32) NOT NULL CHECK (event_type IN ('happy_hour', 'bloodbath')),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_nexus_coude_events_active
    ON nexus_coude_events (guild_id, event_type) WHERE active = TRUE;
