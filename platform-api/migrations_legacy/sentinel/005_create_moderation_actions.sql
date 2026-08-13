CREATE TABLE IF NOT EXISTS moderation_actions (
    id              UUID PRIMARY KEY,
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    moderator_id    TEXT NOT NULL,
    moderator_name  TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    target_name     TEXT NOT NULL,
    action_type     TEXT NOT NULL,
    reason          TEXT NOT NULL,
    gravity         TEXT,
    duration        BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mod_actions_guild ON moderation_actions (guild_id);
CREATE INDEX IF NOT EXISTS idx_mod_actions_target ON moderation_actions (guild_id, target_id);
