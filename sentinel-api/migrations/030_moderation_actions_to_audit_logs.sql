-- Finalise la migration des actions de moderation vers audit_logs.
-- L'UUID metier devient directement audit_logs.id ; created_at complete la
-- cle etrangere car audit_logs est partitionnee sur cette colonne.

-- 1. Garantir une ligne d'audit pour chaque action historique encore absente.
INSERT INTO public.audit_logs (
    id, guild_id, event_type, actor_id, actor_name,
    target_id, target_name, channel_id, channel_name, details, created_at
)
SELECT
    ma.id,
    ma.guild_id,
    'mod_' || ma.action_type,
    ma.moderator_id,
    ma.moderator_name,
    ma.target_id,
    ma.target_name,
    ma.channel_id,
    NULL,
    jsonb_build_object(
        'reason', ma.reason,
        'gravity', ma.gravity,
        'duration_secs', ma.duration,
        'action_id', ma.id::text,
        'backfilled', true
    ),
    ma.created_at
FROM public.moderation_actions ma
WHERE NOT EXISTS (
    SELECT 1
    FROM public.audit_logs al
    WHERE al.event_type LIKE 'mod_%'
      AND (al.id = ma.id OR al.details->>'action_id' = ma.id::text)
);

-- 2. Toute action moderee recoit un action_id UUID valide. Les anciens
-- evenements qui n'en avaient pas prennent leur id courant comme identifiant.
UPDATE public.audit_logs
SET details = jsonb_set(
    COALESCE(details, '{}'::jsonb),
    '{action_id}',
    to_jsonb(id::text),
    true
)
WHERE event_type LIKE 'mod_%'
  AND (
      details->>'action_id' IS NULL
      OR details->>'action_id' !~* '^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'
  );

-- 3. Les phases de dual-write/backfill ont pu creer plusieurs traces pour le
-- meme action_id. On conserve la plus ancienne avant de canoniser l'id.
WITH ranked AS (
    SELECT
        tableoid AS partition_oid,
        ctid AS row_id,
        ROW_NUMBER() OVER (
            PARTITION BY details->>'action_id'
            ORDER BY created_at ASC, id ASC
        ) AS rank
    FROM public.audit_logs
    WHERE event_type LIKE 'mod_%'
)
DELETE FROM public.audit_logs al
USING ranked duplicate
WHERE al.tableoid = duplicate.partition_oid
  AND al.ctid = duplicate.row_id
  AND duplicate.rank > 1;

UPDATE public.audit_logs
SET id = (details->>'action_id')::uuid
WHERE event_type LIKE 'mod_%'
  AND id <> (details->>'action_id')::uuid;

-- 4. Migrer les relations vers la vraie cle de la table partitionnee.
ALTER TABLE public.review_queue
    ADD COLUMN action_created_at timestamp with time zone;

ALTER TABLE public.moderation_evidence
    ADD COLUMN action_created_at timestamp with time zone;

UPDATE public.review_queue relation
SET action_created_at = (
    SELECT al.created_at
    FROM public.audit_logs al
    WHERE al.id = relation.action_id AND al.event_type LIKE 'mod_%'
    ORDER BY al.created_at DESC
    LIMIT 1
);

UPDATE public.moderation_evidence relation
SET action_created_at = (
    SELECT al.created_at
    FROM public.audit_logs al
    WHERE al.id = relation.action_id AND al.event_type LIKE 'mod_%'
    ORDER BY al.created_at DESC
    LIMIT 1
);

ALTER TABLE public.review_queue
    ALTER COLUMN action_created_at SET NOT NULL;

ALTER TABLE public.moderation_evidence
    ALTER COLUMN action_created_at SET NOT NULL;

ALTER TABLE public.review_queue
    DROP CONSTRAINT review_queue_action_id_fkey;

ALTER TABLE public.moderation_evidence
    DROP CONSTRAINT moderation_evidence_action_id_fkey;

ALTER TABLE public.review_queue
    ADD CONSTRAINT review_queue_audit_action_fkey
    FOREIGN KEY (action_id, action_created_at)
    REFERENCES public.audit_logs (id, created_at)
    ON DELETE CASCADE;

ALTER TABLE public.moderation_evidence
    ADD CONSTRAINT moderation_evidence_audit_action_fkey
    FOREIGN KEY (action_id, action_created_at)
    REFERENCES public.audit_logs (id, created_at)
    ON DELETE CASCADE;

DROP INDEX public.idx_review_queue_action;
CREATE INDEX idx_review_queue_action
    ON public.review_queue (action_id, action_created_at);

DROP INDEX public.idx_evidence_action;
CREATE INDEX idx_evidence_action
    ON public.moderation_evidence (action_id, action_created_at);

-- Plus aucun lecteur, ecrivain ni FK ne depend de la table historique.
DROP TABLE public.moderation_actions;
