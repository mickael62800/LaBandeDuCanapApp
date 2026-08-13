-- Config par serveur
CREATE TABLE IF NOT EXISTS conduct_config (
    guild_id        TEXT PRIMARY KEY,
    max_points      INT NOT NULL DEFAULT 12,
    regen_amount    INT NOT NULL DEFAULT 1,
    regen_interval  TEXT NOT NULL DEFAULT 'weekly',
    penalty_warn    INT NOT NULL DEFAULT 1,
    penalty_delete  INT NOT NULL DEFAULT 2,
    penalty_mute    INT NOT NULL DEFAULT 3,
    penalty_ban     INT NOT NULL DEFAULT 6,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Points par utilisateur par serveur
CREATE TABLE IF NOT EXISTS user_conduct_points (
    id              UUID PRIMARY KEY,
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    points          INT NOT NULL DEFAULT 12,
    last_regen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id)
);

-- Historique des mouvements de points
CREATE TABLE IF NOT EXISTS conduct_points_log (
    id              UUID PRIMARY KEY,
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    delta           INT NOT NULL,
    reason          TEXT NOT NULL,
    points_before   INT NOT NULL,
    points_after    INT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_conduct_points_guild_user ON user_conduct_points (guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_conduct_log_guild_user ON conduct_points_log (guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_conduct_log_created ON conduct_points_log (created_at DESC);
