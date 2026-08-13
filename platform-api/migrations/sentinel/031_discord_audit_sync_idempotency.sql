-- Identite dediee et idempotence forte pour les entrees Discord Audit.
-- `created_at` fait partie de l'unicite car audit_logs est partitionnee dessus.
ALTER TABLE public.audit_logs
    ADD COLUMN discord_entry_id text;

UPDATE public.audit_logs
SET discord_entry_id = details->>'discord_entry_id'
WHERE event_type LIKE 'discord_audit:%'
  AND details->>'discord_entry_id' ~ '^[0-9]+$';

-- Corrige aussi l'historique : l'heure est encodee dans les 42 bits hauts du
-- snowflake, en millisecondes depuis l'epoch Discord.
UPDATE public.audit_logs
SET created_at = to_timestamp((
    (floor(discord_entry_id::numeric / 4194304) + 1420070400000) / 1000.0
)::double precision)
WHERE discord_entry_id IS NOT NULL;

-- Une remise a zero du curseur a pu importer plusieurs fois la meme entree.
WITH ranked AS (
    SELECT
        tableoid AS partition_oid,
        ctid AS row_id,
        ROW_NUMBER() OVER (
            PARTITION BY discord_entry_id
            ORDER BY created_at ASC, id ASC
        ) AS rank
    FROM public.audit_logs
    WHERE discord_entry_id IS NOT NULL
)
DELETE FROM public.audit_logs al
USING ranked duplicate
WHERE al.tableoid = duplicate.partition_oid
  AND al.ctid = duplicate.row_id
  AND duplicate.rank > 1;

CREATE UNIQUE INDEX uq_audit_logs_discord_entry
    ON public.audit_logs (discord_entry_id, created_at)
    WHERE discord_entry_id IS NOT NULL;
