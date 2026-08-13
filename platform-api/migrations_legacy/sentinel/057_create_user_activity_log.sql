-- Systeme de surveillance active : log d'activite des utilisateurs surveilles
CREATE TABLE IF NOT EXISTS user_activity_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    channel_id      TEXT,
    channel_name    TEXT,
    content         TEXT,
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_activity_guild_user ON user_activity_log (guild_id, user_id);
CREATE INDEX idx_user_activity_created ON user_activity_log (created_at);
CREATE INDEX idx_user_activity_type ON user_activity_log (event_type);
