-- Migration 002 : promotion du wallet en systeme partage Nexus.
--
-- - `username` sur nexus_wallets (l'ancien schema Sentinel
--   `080_create_user_wallets.sql` l'avait : TEXT NOT NULL DEFAULT '').
-- - `reason` optionnelle sur nexus_wallet_transactions (motif libre saisi
--   par le joueur/admin, distinct de la description technique).
-- - Config par guild du solde de depart (`starting_coins`, defaut
--   historique 100, cf. ancienne migration 285).
--
-- L'index leaderboard (guild_id, coins DESC) existe deja depuis 001
-- (idx_nexus_wallets_guild_coins) ; recree en IF NOT EXISTS par surete.

ALTER TABLE nexus_wallets
    ADD COLUMN IF NOT EXISTS username VARCHAR(100) NOT NULL DEFAULT '';

ALTER TABLE nexus_wallet_transactions
    ADD COLUMN IF NOT EXISTS reason TEXT;

CREATE INDEX IF NOT EXISTS idx_nexus_wallets_guild_coins
    ON nexus_wallets (guild_id, coins DESC);

CREATE TABLE IF NOT EXISTS nexus_guild_config (
    guild_id       VARCHAR(20) PRIMARY KEY,
    starting_coins BIGINT NOT NULL DEFAULT 100 CHECK (starting_coins >= 0),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
