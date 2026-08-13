-- Phase Sync — Automod review cards
--
-- Persiste les cartes de review automod postees par le bot dans le
-- channel modération. Permet à la web de :
--   * lister les reviews en attente (`status = 'pending'`)
--   * appliquer une action (warn/mute/ban/delete) ou ignorer depuis le web
--   * recevoir l'event de resolution via WebSocket
-- Le bot edite la carte Discord en parallele (greyed-out + footer "via web").

CREATE TABLE IF NOT EXISTS automod_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    user_name TEXT NOT NULL,
    content_preview TEXT NOT NULL,
    suggested_action TEXT NOT NULL CHECK (suggested_action IN ('warn','delete','mute','ban')),
    score DOUBLE PRECISION NOT NULL DEFAULT 0,
    reason TEXT NOT NULL DEFAULT '',
    flags JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending','applied','ignored')),
    applied_action TEXT CHECK (applied_action IN ('warn','delete','mute','ban','ignore')),
    resolved_by_id TEXT,
    resolved_by_name TEXT,
    resolved_source TEXT CHECK (resolved_source IN ('discord','web')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_automod_reviews_guild_status
    ON automod_reviews (guild_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_automod_reviews_user
    ON automod_reviews (user_id);
