-- Phase 6B wave 3 — MOD #2 /evidence
--
-- Table pour attacher des preuves (URLs, captures d'ecran, logs) a une action
-- de moderation existante. Permet au moderateur de justifier a posteriori ses
-- decisions, et a l'appelant d'avoir le contexte complet.

CREATE TABLE IF NOT EXISTS moderation_evidence (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action_id UUID NOT NULL REFERENCES moderation_actions(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    description TEXT,
    uploaded_by VARCHAR(20) NOT NULL,
    uploaded_by_name TEXT NOT NULL,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_evidence_action ON moderation_evidence (action_id);
CREATE INDEX IF NOT EXISTS idx_evidence_uploaded_at ON moderation_evidence (uploaded_at DESC);
