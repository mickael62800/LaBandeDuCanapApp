-- Phase 6A — appeal-sla-worker
--
-- Ajoute une colonne `escalated_at` a la table `tickets` pour marquer les
-- tickets d'appel de sanction qui ont depasse le SLA de premiere reponse et
-- ete escalades par le worker.
--
-- Les colonnes `first_response_at` et `resolved_at` existent deja (migration
-- 053). Le SLA en minutes est lu depuis `bot_guild_config` via les cles
-- `sla_first_response_minutes` et `sla_escalation_minutes` definies dans la
-- migration 047 (defaut 30min et 60min respectivement).

ALTER TABLE tickets ADD COLUMN IF NOT EXISTS escalated_at TIMESTAMPTZ;

-- Index partiel : le worker scan uniquement les tickets ouverts d'appel
-- sans escalade. Hors de ces conditions, l'index ne sert a rien.
CREATE INDEX IF NOT EXISTS idx_tickets_appeal_sla_pending
    ON tickets (created_at, server)
    WHERE category = 'appel_sanction'
      AND status IN ('open', 'assigned')
      AND escalated_at IS NULL
      AND first_response_at IS NULL;
