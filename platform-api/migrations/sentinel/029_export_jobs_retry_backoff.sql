-- Rend les echecs transitoires d'export a nouveau eligibles sans boucle chaude.
-- NULL signifie "immediatement eligible" pour les jobs existants et nouveaux.
ALTER TABLE public.export_jobs
    ADD COLUMN next_attempt_at timestamp with time zone;

DROP INDEX public.idx_export_jobs_pending;

CREATE INDEX idx_export_jobs_pending
    ON public.export_jobs (
        COALESCE(next_attempt_at, created_at),
        created_at
    )
    WHERE status = 'pending';
