-- Ticket SLA warning : nouvelle colonne pour tracker l'envoi du warning
-- (avant escalation) et eviter les doublons sur scans repetes.
--
-- Flow :
--   age >= sla_first_response_minutes ET sla_warned_at IS NULL ET
--   first_response_at IS NULL -> publish ticket_sla_warned + UPDATE
--   sla_warned_at = NOW(). Le bot consomme et poste un message.
--
-- L'escalation reelle (priority high) reste declenchee par
-- sla_escalation_minutes dans escalate_sla.rs.

ALTER TABLE tickets
    ADD COLUMN IF NOT EXISTS sla_warned_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_tickets_sla_warning
    ON tickets (server, created_at)
    WHERE first_response_at IS NULL
      AND sla_warned_at IS NULL
      AND escalated_at IS NULL
      AND status IN ('open', 'assigned')
      AND category != 'appel_sanction';
