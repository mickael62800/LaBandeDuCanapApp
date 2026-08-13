-- Temporary voice channels (persistent state)
CREATE TABLE IF NOT EXISTS voice_channels (
    id              UUID PRIMARY KEY,
    guild_id        TEXT NOT NULL,
    owner_id        TEXT NOT NULL,
    owner_name      TEXT NOT NULL,
    channel_id      TEXT NOT NULL UNIQUE,
    text_channel_id TEXT,
    members_channel_id TEXT,
    queue_channel_id TEXT,
    category_id     TEXT,
    channel_name    TEXT NOT NULL,
    kind            TEXT NOT NULL DEFAULT 'public',
    visibility      TEXT NOT NULL DEFAULT 'visible',
    queue_enabled   BOOLEAN NOT NULL DEFAULT FALSE,
    locked          BOOLEAN NOT NULL DEFAULT FALSE,
    member_limit    INT,
    status          TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Co-admins for temp voice channels
CREATE TABLE IF NOT EXISTS voice_channel_co_admins (
    id               UUID PRIMARY KEY,
    voice_channel_id UUID NOT NULL REFERENCES voice_channels(id) ON DELETE CASCADE,
    user_id          TEXT NOT NULL,
    user_name        TEXT NOT NULL,
    granted_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(voice_channel_id, user_id)
);

-- Persistent per-owner whitelist (friend list across sessions)
CREATE TABLE IF NOT EXISTS voice_channel_whitelists (
    id          UUID PRIMARY KEY,
    guild_id    TEXT NOT NULL,
    owner_id    TEXT NOT NULL,
    target_id   TEXT NOT NULL,
    target_name TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, owner_id, target_id)
);

-- Temporary and permanent bans from voice channels
CREATE TABLE IF NOT EXISTS voice_channel_bans (
    id               UUID PRIMARY KEY,
    voice_channel_id UUID NOT NULL REFERENCES voice_channels(id) ON DELETE CASCADE,
    user_id          TEXT NOT NULL,
    user_name        TEXT NOT NULL,
    banned_by        TEXT NOT NULL,
    reason           TEXT,
    expires_at       TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(voice_channel_id, user_id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_voice_channels_guild ON voice_channels (guild_id);
CREATE INDEX IF NOT EXISTS idx_voice_channels_owner ON voice_channels (owner_id);
CREATE INDEX IF NOT EXISTS idx_voice_co_admins_channel ON voice_channel_co_admins (voice_channel_id);
CREATE INDEX IF NOT EXISTS idx_voice_whitelists_owner ON voice_channel_whitelists (guild_id, owner_id);
CREATE INDEX IF NOT EXISTS idx_voice_bans_channel ON voice_channel_bans (voice_channel_id);
CREATE INDEX IF NOT EXISTS idx_voice_bans_expires ON voice_channel_bans (expires_at) WHERE expires_at IS NOT NULL;
