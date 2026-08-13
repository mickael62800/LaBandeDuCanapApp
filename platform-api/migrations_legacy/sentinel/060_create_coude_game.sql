-- ============================================
-- Coup de Coude — Mini-jeu social chaotique
-- ============================================

-- Joueurs
CREATE TABLE IF NOT EXISTS coude_players (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    class           TEXT NOT NULL DEFAULT 'bourrin',  -- bourrin, agile, fourbe, tank
    coins           BIGINT NOT NULL DEFAULT 100,
    total_wins      INT NOT NULL DEFAULT 0,
    total_losses    INT NOT NULL DEFAULT 0,
    total_draws     INT NOT NULL DEFAULT 0,
    total_earned    BIGINT NOT NULL DEFAULT 0,
    total_lost      BIGINT NOT NULL DEFAULT 0,
    total_stolen    BIGINT NOT NULL DEFAULT 0,
    cowardice_count INT NOT NULL DEFAULT 0,          -- nombre de refus
    casino_wins     INT NOT NULL DEFAULT 0,
    casino_losses   INT NOT NULL DEFAULT 0,
    chaos_events    INT NOT NULL DEFAULT 0,          -- nombre d'evenements chaos subis
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_coude_players_guild ON coude_players(guild_id);
CREATE INDEX IF NOT EXISTS idx_coude_players_coins ON coude_players(guild_id, coins DESC);

-- Combats (en attente + historique)
CREATE TABLE IF NOT EXISTS coude_combats (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    attacker_id     TEXT NOT NULL,
    attacker_name   TEXT NOT NULL,
    defender_id     TEXT NOT NULL,
    defender_name   TEXT NOT NULL,
    mise            BIGINT NOT NULL DEFAULT 10,
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending, accepted, refused, expired
    winner_id       TEXT,
    attacker_roll   INT,
    defender_roll   INT,
    chaos_event     TEXT,                             -- nom de l'evenement chaos (null si aucun)
    special_attack  TEXT,                             -- surprise, double_coup, coup_traitre
    result_message  TEXT,
    coins_transferred BIGINT DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_coude_combats_guild ON coude_combats(guild_id, status);
CREATE INDEX IF NOT EXISTS idx_coude_combats_pending ON coude_combats(defender_id, status) WHERE status = 'pending';

-- Primes (bounties)
CREATE TABLE IF NOT EXISTS coude_primes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    target_name     TEXT NOT NULL,
    placed_by_id    TEXT NOT NULL,
    placed_by_name  TEXT NOT NULL,
    amount          BIGINT NOT NULL,
    claimed         BOOLEAN NOT NULL DEFAULT FALSE,
    claimed_by_id   TEXT,
    claimed_by_name TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    claimed_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_coude_primes_target ON coude_primes(guild_id, target_id, claimed);

-- Inventaire (objets achetes)
CREATE TABLE IF NOT EXISTS coude_inventory (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    item_key        TEXT NOT NULL,       -- explosion, inversion, mindgame, rage, surprise, double_coup, coup_traitre
    quantity        INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id, item_key)
);

-- Evenements serveur actifs
CREATE TABLE IF NOT EXISTS coude_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    event_type      TEXT NOT NULL,       -- happy_hour, bloodbath, drop
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_coude_events_active ON coude_events(guild_id, active) WHERE active = TRUE;
