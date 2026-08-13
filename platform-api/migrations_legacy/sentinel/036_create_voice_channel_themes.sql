CREATE TABLE IF NOT EXISTS voice_channel_themes (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id              TEXT NOT NULL,
    name                  TEXT NOT NULL,
    emoji                 TEXT,
    channel_name_template TEXT NOT NULL DEFAULT '{user}',
    member_limit          INT,
    visibility            TEXT NOT NULL DEFAULT 'visible',
    locked                BOOLEAN NOT NULL DEFAULT FALSE,
    queue_enabled         BOOLEAN NOT NULL DEFAULT FALSE,
    bitrate               INT,
    slowmode_secs         INT,
    is_default            BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order            INT NOT NULL DEFAULT 0,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, name)
);

CREATE INDEX idx_voice_themes_guild ON voice_channel_themes (guild_id);
