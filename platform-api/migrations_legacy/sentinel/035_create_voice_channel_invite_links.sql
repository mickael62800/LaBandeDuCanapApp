CREATE TABLE IF NOT EXISTS voice_channel_invite_links (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    voice_channel_id UUID NOT NULL REFERENCES voice_channels(id) ON DELETE CASCADE,
    guild_id         TEXT NOT NULL,
    channel_id       TEXT NOT NULL,
    created_by       TEXT NOT NULL,
    created_by_name  TEXT NOT NULL,
    code             TEXT NOT NULL UNIQUE,
    max_uses         INT,
    current_uses     INT NOT NULL DEFAULT 0,
    expires_at       TIMESTAMPTZ NOT NULL,
    revoked          BOOLEAN NOT NULL DEFAULT FALSE,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_invite_links_channel ON voice_channel_invite_links (channel_id);
CREATE INDEX idx_invite_links_voice_channel ON voice_channel_invite_links (voice_channel_id);
CREATE INDEX idx_invite_links_expires ON voice_channel_invite_links (expires_at);
