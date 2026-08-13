-- Phase 6B wave 3 — MOD #3 /review
--
-- File de relecture des actions de moderation sensibles. Permet a un moderateur
-- de demander une seconde opinion sur une action deja appliquee. Les
-- moderateurs seniors peuvent ensuite parcourir la queue et approuver/rejeter
-- (avec notes pour le demandeur).
--
-- Statuts possibles : 'pending' (en attente), 'approved' (valide),
-- 'rejected' (action jugee inappropriee), 'changed' (action a modifier).

CREATE TABLE IF NOT EXISTS review_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action_id UUID NOT NULL REFERENCES moderation_actions(id) ON DELETE CASCADE,
    guild_id VARCHAR(20) NOT NULL,
    added_by VARCHAR(20) NOT NULL,
    added_by_name TEXT NOT NULL,
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'changed')),
    reviewer_id VARCHAR(20),
    reviewer_name TEXT,
    reviewer_notes TEXT,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Index partiel : la plupart des lookups sont sur les reviews pending.
CREATE INDEX IF NOT EXISTS idx_review_queue_pending
    ON review_queue (guild_id, added_at DESC)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_review_queue_action ON review_queue (action_id);
