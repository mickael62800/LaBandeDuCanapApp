-- Systeme de strikes / escalade progressive

CREATE TABLE strike_config (
    guild_id     TEXT NOT NULL,
    window_secs  BIGINT NOT NULL DEFAULT 3600,
    thresholds   JSONB NOT NULL DEFAULT '[]',
    enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id)
);

CREATE TABLE user_strikes (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id      TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    reason        TEXT NOT NULL,
    source        TEXT NOT NULL,
    infraction_id UUID,
    expires_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_strikes_guild_user ON user_strikes(guild_id, user_id);
CREATE INDEX idx_strikes_expires ON user_strikes(expires_at) WHERE expires_at IS NOT NULL;
