-- Ajout des champs pour la phase de paris (betting)
ALTER TABLE coude_combats ADD COLUMN IF NOT EXISTS accepted_at TIMESTAMPTZ;
ALTER TABLE coude_combats ADD COLUMN IF NOT EXISTS message_id TEXT;

CREATE INDEX IF NOT EXISTS idx_coude_combats_betting
    ON coude_combats(status, accepted_at) WHERE status = 'betting';
