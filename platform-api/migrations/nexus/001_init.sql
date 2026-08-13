-- Migration 001 : init Nexus — wallet + Roue du Destin.
--
-- Colonnes reprises des anciennes migrations Sentinel
-- (080_create_user_wallets.sql, 158_create_wheel_of_destiny.sql),
-- prefixees `nexus_` et nettoyees (pas de bot_definitions ici : la config
-- par serveur viendra plus tard).

-- ── Wallet partage entre les futurs jeux Nexus ──

CREATE TABLE IF NOT EXISTS nexus_wallets (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     VARCHAR(20) NOT NULL,
    user_id      VARCHAR(20) NOT NULL,
    coins        BIGINT NOT NULL DEFAULT 0 CHECK (coins >= 0),
    total_earned BIGINT NOT NULL DEFAULT 0,
    total_spent  BIGINT NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_nexus_wallets_guild_coins
    ON nexus_wallets (guild_id, coins DESC);

-- Historique de toutes les operations (positif = credit, negatif = debit).
CREATE TABLE IF NOT EXISTS nexus_wallet_transactions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id      VARCHAR(20) NOT NULL,
    user_id       VARCHAR(20) NOT NULL,
    amount        BIGINT NOT NULL,
    balance_after BIGINT NOT NULL,
    source        VARCHAR(40) NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_nexus_wallet_tx_user
    ON nexus_wallet_transactions (guild_id, user_id, created_at DESC);

-- ── Roue du Destin ──

-- Historique des spins. Une row par spin.
CREATE TABLE IF NOT EXISTS nexus_wheel_spin_log (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id   VARCHAR(20) NOT NULL,
    user_id    VARCHAR(20) NOT NULL,
    username   VARCHAR(100) NOT NULL,
    case_key   VARCHAR(40) NOT NULL,
    case_label VARCHAR(100) NOT NULL,
    payout     BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_nexus_wheel_spin_log_guild_created
    ON nexus_wheel_spin_log (guild_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_nexus_wheel_spin_log_user_guild
    ON nexus_wheel_spin_log (guild_id, user_id, created_at DESC);

-- Tracking du daily : 1 row par (guild, user, day). Existence = deja claim.
CREATE TABLE IF NOT EXISTS nexus_wheel_daily_claims (
    guild_id   VARCHAR(20) NOT NULL,
    user_id    VARCHAR(20) NOT NULL,
    day        DATE        NOT NULL,
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id, day)
);
