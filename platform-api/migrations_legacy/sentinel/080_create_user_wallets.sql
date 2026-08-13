-- ============================================
-- Wallet partage — systeme de coins unifie entre tous les jeux
-- ============================================

CREATE TABLE IF NOT EXISTS user_wallets (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    username    TEXT NOT NULL DEFAULT '',
    coins       BIGINT NOT NULL DEFAULT 0,
    total_earned BIGINT NOT NULL DEFAULT 0,
    total_spent  BIGINT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_wallets_guild ON user_wallets(guild_id);
CREATE INDEX IF NOT EXISTS idx_wallets_coins ON user_wallets(guild_id, coins DESC);

-- Transactions log (historique de toutes les operations)
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    amount      BIGINT NOT NULL,          -- positif = credit, negatif = debit
    balance_after BIGINT NOT NULL,
    source      TEXT NOT NULL,             -- 'blackjack', 'coude', 'casino', 'admin', 'daily', etc.
    description TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_wallet_tx_user ON wallet_transactions(guild_id, user_id, created_at DESC);

-- Migration : copier les coins existants de coude_players vers user_wallets
INSERT INTO user_wallets (guild_id, user_id, username, coins, total_earned, total_spent, created_at, updated_at)
SELECT guild_id, user_id, username, coins, total_earned, total_lost, created_at, updated_at
FROM coude_players
ON CONFLICT (guild_id, user_id) DO UPDATE SET
    coins = EXCLUDED.coins,
    total_earned = EXCLUDED.total_earned,
    total_spent = EXCLUDED.total_spent;
