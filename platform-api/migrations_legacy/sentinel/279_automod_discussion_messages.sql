-- Transcript des salons de discussion automod : on capture la conversation au
-- moment de l'archivage (finalisation / clôture) pour en garder une TRACE
-- consultable sur le web, même si le salon Discord est ensuite supprimé.

CREATE TABLE IF NOT EXISTS automod_discussion_messages (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id           UUID NOT NULL REFERENCES automod_reviews(id) ON DELETE CASCADE,
    discord_message_id  TEXT NOT NULL,
    author_id           TEXT NOT NULL,
    author_name         TEXT NOT NULL DEFAULT '',
    author_is_bot       BOOLEAN NOT NULL DEFAULT FALSE,
    content             TEXT NOT NULL DEFAULT '',
    sent_at             TIMESTAMPTZ NOT NULL,
    captured_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Idempotent : un même message n'est stocké qu'une fois par review.
    UNIQUE (review_id, discord_message_id)
);

CREATE INDEX IF NOT EXISTS idx_automod_disc_msgs_review
    ON automod_discussion_messages (review_id, sent_at);
