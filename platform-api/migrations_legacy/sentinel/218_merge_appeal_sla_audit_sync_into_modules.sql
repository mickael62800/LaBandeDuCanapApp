-- Phase composants — Fusions finales :
--   1. appeal_sla -> ticket-bot (le SLA des appels concerne les tickets)
--   2. discord_audit_sync -> audit-bot (sync des audit logs Discord)

-- ── 1. appeal_sla -> ticket-bot ────────────────────────────────────
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'appeal_sla'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'ticket-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'ticket-bot'
    WHERE bot_name = 'appeal_sla';

-- Append des cles SLA au schema ticket-bot (idempotent : on filtre les
-- cles deja presentes).
UPDATE bot_definitions
   SET config_schema = config_schema || '[
        {"key": "appeal_sla_scan_interval", "label": "Worker : intervalle scan SLA appels", "type": "number", "required": false, "default": "300", "min": 30, "max": 3600, "unit": "s", "description": "Frequence de scan des tickets d appel pour detection de depassement SLA. Le worker remonte une alerte aux superviseurs.", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
 WHERE bot_name = 'ticket-bot'
   AND NOT (config_schema @> '[{"key": "appeal_sla_scan_interval"}]'::jsonb);

DELETE FROM bot_definitions WHERE bot_name = 'appeal_sla';

-- ── 2. discord_audit_sync -> audit-bot ─────────────────────────────
DELETE FROM bot_guild_config wkr
    WHERE wkr.bot_name = 'discord_audit_sync'
      AND EXISTS (
          SELECT 1 FROM bot_guild_config m
           WHERE m.bot_name = 'audit-bot'
             AND m.guild_id = wkr.guild_id
             AND m.config_key = wkr.config_key
      );
UPDATE bot_guild_config SET bot_name = 'audit-bot'
    WHERE bot_name = 'discord_audit_sync';

UPDATE bot_definitions
   SET config_schema = config_schema || '[
        {"key": "audit_sync_interval", "label": "Worker : intervalle sync audit Discord", "type": "number", "required": false, "default": "300", "min": 60, "max": 3600, "unit": "s", "description": "Frequence de polling des audit logs Discord via l API REST (rattrape les events rates en gateway).", "depends_on": {"key": "enabled", "equals": "true"}}
    ]'::jsonb
 WHERE bot_name = 'audit-bot'
   AND NOT (config_schema @> '[{"key": "audit_sync_interval"}]'::jsonb);

DELETE FROM bot_definitions WHERE bot_name = 'discord_audit_sync';
