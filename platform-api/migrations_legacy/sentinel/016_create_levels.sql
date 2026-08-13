-- Configuration XP par serveur
CREATE TABLE IF NOT EXISTS level_config (
    guild_id TEXT PRIMARY KEY,
    xp_per_message INTEGER NOT NULL DEFAULT 15,
    xp_per_voice_minute INTEGER NOT NULL DEFAULT 5,
    xp_cooldown_secs INTEGER NOT NULL DEFAULT 60,
    level_up_channel_id TEXT,
    level_up_message TEXT NOT NULL DEFAULT 'GG {user}, tu es maintenant niveau **{level}** !',
    excluded_channels TEXT[] NOT NULL DEFAULT '{}',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Niveaux des utilisateurs
CREATE TABLE IF NOT EXISTS user_levels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    username TEXT NOT NULL DEFAULT '',
    xp BIGINT NOT NULL DEFAULT 0,
    level INTEGER NOT NULL DEFAULT 0,
    last_xp_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_user_levels_guild_user UNIQUE (guild_id, user_id)
);

-- Roles-recompenses par palier
CREATE TABLE IF NOT EXISTS level_rewards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    level INTEGER NOT NULL,
    role_id TEXT NOT NULL,
    CONSTRAINT uq_level_rewards_guild_level UNIQUE (guild_id, level)
);

CREATE INDEX idx_user_levels_guild ON user_levels (guild_id);
CREATE INDEX idx_user_levels_guild_xp ON user_levels (guild_id, xp DESC);
CREATE INDEX idx_user_levels_guild_level ON user_levels (guild_id, level DESC);
CREATE INDEX idx_level_rewards_guild ON level_rewards (guild_id);
