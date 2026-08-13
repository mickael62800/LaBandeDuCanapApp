-- Phase 4 A — Table de file d'attente des jobs IA
--
-- Architecture asynchrone : les bots POSTent un job (retour 202 immediat),
-- l'ai-worker depile via un periodic scan, appelle l'API d'inference, persiste
-- le resultat et publie sur Redis pour reveiller les consommateurs.
--
-- Champs cles :
--   - status : pending → processing → done | failed | dead
--   - retries : compteur de retry; depasse max → 'dead' (DLQ logique)
--   - cost_tokens : tracking par guild pour quota/billing futur
--   - input_payload / result_payload : JSONB libres, le worker les sait parser

CREATE TABLE IF NOT EXISTS ai_jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        VARCHAR(20) NOT NULL,
    job_type        TEXT NOT NULL,                 -- 'analyze_text' | 'analyze_image'
    status          TEXT NOT NULL DEFAULT 'pending', -- pending|processing|done|failed|dead
    input_payload   JSONB NOT NULL,
    result_payload  JSONB,
    error_message   TEXT,
    retries         INT NOT NULL DEFAULT 0,
    max_retries     INT NOT NULL DEFAULT 3,
    cost_tokens     BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    CONSTRAINT chk_ai_jobs_status CHECK (status IN ('pending','processing','done','failed','dead')),
    CONSTRAINT chk_ai_jobs_type CHECK (job_type IN ('analyze_text','analyze_image'))
);

-- Index hot path : worker polling pending jobs ASC
CREATE INDEX IF NOT EXISTS idx_ai_jobs_pending
    ON ai_jobs (created_at)
    WHERE status = 'pending';

-- Index pour le tracking par guild (futur quota/billing)
CREATE INDEX IF NOT EXISTS idx_ai_jobs_guild_created
    ON ai_jobs (guild_id, created_at DESC);

-- Index pour la query "jobs en cours qui trainent" (timeout detector)
CREATE INDEX IF NOT EXISTS idx_ai_jobs_processing
    ON ai_jobs (started_at)
    WHERE status = 'processing';
