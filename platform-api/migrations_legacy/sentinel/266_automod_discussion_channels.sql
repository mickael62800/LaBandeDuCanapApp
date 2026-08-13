-- Automod — salons de discussion lies a une review (bouton "Ouvrir une
-- discussion"). Persiste pour l'audit et l'idempotence (un seul salon par
-- review). Le bot cree le salon Discord puis enregistre la trace ici via
-- l'API ; le web peut ainsi savoir qu'un salon existe.

CREATE TABLE IF NOT EXISTS automod_discussion_channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id UUID NOT NULL REFERENCES automod_reviews(id) ON DELETE CASCADE,
    guild_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    opened_by_id TEXT NOT NULL,
    opened_by_name TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Idempotence : un seul salon de discussion par review.
    UNIQUE (review_id)
);

CREATE INDEX IF NOT EXISTS idx_automod_discussion_channels_review
    ON automod_discussion_channels (review_id);
