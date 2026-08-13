-- Phase 2 A.2 — Vues materialisees pour les leaderboards + user_cache
--
-- Probleme : les leaderboards (coude, wallets, levels) sont consultes en
-- permanence par le dashboard et par les commandes Discord. La query
-- "SELECT ... ORDER BY coins DESC LIMIT N" doit trier toutes les lignes de
-- la guild a chaque appel — quelques milliers de joueurs * dizaines de hits
-- par minute = stress permanent sur le buffer pool.
--
-- Solution : 3 vues materialisees, refreshees toutes les 5 minutes par le
-- cache-worker. Lecture O(1) avec index sur (guild_id, rank). Gain typique :
-- 100-1000x sur les hits leaderboard.
--
-- En complement : table `user_cache` qui devient la source unique de verite
-- pour les usernames Discord. Les colonnes `username` denormalisees dans
-- les tables hot (coude_players, user_wallets, user_levels, user_stats)
-- pourront a terme etre repointees vers user_cache, mais pour l'instant on
-- continue a les remplir directement (changement non-breaking).

-- ── Vues materialisees ───────────────────────────────────────────────────────

-- Coude leaderboard : tri par coins, avec rang precalcule
-- On copie TOUTES les colonnes de coude_players pour que la MV soit un drop-in
-- replacement complet de la table dans la query `list()` du repository.
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_coude_leaderboard AS
SELECT
    guild_id,
    user_id,
    username,
    coins,
    total_wins,
    total_losses,
    total_draws,
    total_earned,
    total_lost,
    total_stolen,
    cowardice_count,
    chaos_events,
    casino_wins,
    casino_losses,
    level,
    xp,
    stat_points,
    atk,
    def,
    class,
    title,
    hp_current,
    hp_max,
    hp_last_regen,
    repos_last_used,
    class_changed_at,
    season,
    created_at,
    updated_at,
    ROW_NUMBER() OVER (PARTITION BY guild_id ORDER BY coins DESC) AS rank
FROM coude_players;

-- Index UNIQUE requis par REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX IF NOT EXISTS uq_mv_coude_leaderboard
    ON mv_coude_leaderboard (guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_mv_coude_leaderboard_rank
    ON mv_coude_leaderboard (guild_id, rank);

-- Wallets leaderboard
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_wallet_leaderboard AS
SELECT
    id,
    guild_id,
    user_id,
    username,
    coins,
    total_earned,
    total_spent,
    ROW_NUMBER() OVER (PARTITION BY guild_id ORDER BY coins DESC) AS rank,
    created_at,
    updated_at
FROM user_wallets;

CREATE UNIQUE INDEX IF NOT EXISTS uq_mv_wallet_leaderboard
    ON mv_wallet_leaderboard (guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_mv_wallet_leaderboard_rank
    ON mv_wallet_leaderboard (guild_id, rank);

-- Levels leaderboard (XP global)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_level_leaderboard AS
SELECT
    id,
    guild_id,
    user_id,
    username,
    xp,
    level,
    xp_text,
    level_text,
    xp_voice,
    level_voice,
    ROW_NUMBER() OVER (PARTITION BY guild_id ORDER BY xp DESC) AS rank,
    last_xp_at,
    created_at,
    updated_at
FROM user_levels;

CREATE UNIQUE INDEX IF NOT EXISTS uq_mv_level_leaderboard
    ON mv_level_leaderboard (guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_mv_level_leaderboard_rank
    ON mv_level_leaderboard (guild_id, rank);

-- ── user_cache : source de verite des usernames Discord ─────────────────────

CREATE TABLE IF NOT EXISTS user_cache (
    guild_id    VARCHAR(20) NOT NULL,
    user_id     VARCHAR(20) NOT NULL,
    username    TEXT NOT NULL,
    avatar_url  TEXT,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_cache_updated
    ON user_cache (updated_at DESC);
