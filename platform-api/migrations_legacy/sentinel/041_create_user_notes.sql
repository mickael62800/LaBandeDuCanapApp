-- Systeme de notes utilisateur (moderation)

CREATE TABLE user_notes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT NOT NULL,
    user_id      TEXT NOT NULL,
    author_id    TEXT NOT NULL,
    author_name  TEXT NOT NULL,
    content      TEXT NOT NULL,
    category     TEXT NOT NULL DEFAULT 'general',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notes_guild_user ON user_notes(guild_id, user_id);
