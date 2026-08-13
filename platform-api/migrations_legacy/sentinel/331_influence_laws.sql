-- Jeu « Influence » — Phase 3 : cycle de loi (depot -> vote -> application).
-- La table influence_laws n'avait pas ete creee en Phase 1 : on la cree ici,
-- avec le suivi du message Discord (edition a la cloture par le worker) et un
-- index de scan pour le worker « monde vivant ». Idempotent.

CREATE TABLE IF NOT EXISTS influence_laws (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'vote',   -- vote|adoptee|rejetee
    author_id   UUID NOT NULL REFERENCES influence_citizens(id) ON DELETE CASCADE,
    effects     JSONB NOT NULL DEFAULT '{}'::jsonb,
    expires_at  TIMESTAMPTZ,                     -- echeance du vote
    channel_id  TEXT,
    message_id  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Colonnes de suivi du message (si la table preexistait sans elles).
ALTER TABLE influence_laws ADD COLUMN IF NOT EXISTS channel_id TEXT;
ALTER TABLE influence_laws ADD COLUMN IF NOT EXISTS message_id TEXT;

-- Scan worker : lois en cours de vote dont l'echeance est passee.
CREATE INDEX IF NOT EXISTS idx_influence_laws_due
    ON influence_laws (expires_at) WHERE status = 'vote';
