-- Phase 6A — Table de file d'attente des jobs d'export
--
-- Architecture identique a ai_jobs (Phase 4 A) : les clients POSTent un job
-- (retour 202 immediat), l'export-worker depile via periodic scan, execute
-- la query appropriee selon job_type, serialize en CSV/JSON, persiste le
-- resultat inline dans `result` (TEXT).
--
-- Resultat stocke inline (pas de disk/S3) pour eviter la complexite :
--   - Les bots recuperent via GET /api/exports/jobs/{id} et envoient comme
--     piece jointe Discord
--   - Pour des exports tres volumineux, un design ulterieur avec storage
--     externe sera necessaire (limite Discord : 10 MB par fichier en free,
--     25 MB boost tier 2)
--
-- Types supportes :
--   - infractions : export infractions d'une guild
--   - audit_logs : export audit_logs d'une guild
--   - moderation_actions : export moderation_actions d'une guild

CREATE TABLE IF NOT EXISTS export_jobs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        VARCHAR(20) NOT NULL,
    requested_by    VARCHAR(20) NOT NULL,
    job_type        TEXT NOT NULL,                 -- 'infractions' | 'audit_logs' | 'moderation_actions'
    format          TEXT NOT NULL,                 -- 'csv' | 'json'
    filters         JSONB NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending', -- pending|processing|done|failed|dead
    result          TEXT,                           -- CSV ou JSON serialise
    result_rows     INT,                            -- nombre de lignes exportees
    error_message   TEXT,
    retries         INT NOT NULL DEFAULT 0,
    max_retries     INT NOT NULL DEFAULT 3,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at      TIMESTAMPTZ,
    completed_at    TIMESTAMPTZ,
    CONSTRAINT chk_export_jobs_status CHECK (status IN ('pending','processing','done','failed','dead')),
    CONSTRAINT chk_export_jobs_type CHECK (job_type IN ('infractions','audit_logs','moderation_actions')),
    CONSTRAINT chk_export_jobs_format CHECK (format IN ('csv','json'))
);

CREATE INDEX IF NOT EXISTS idx_export_jobs_pending
    ON export_jobs (created_at)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_export_jobs_guild_created
    ON export_jobs (guild_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_export_jobs_processing
    ON export_jobs (started_at)
    WHERE status = 'processing';
